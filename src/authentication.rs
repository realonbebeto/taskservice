use crate::domain::id::ProfileId;
use crate::error::authentication::{AuthError, StdResponse};
use crate::session_state::TypedSession;
use crate::startup::SecretKey;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::util::e500;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{
    FromRequest, HttpResponse, http::header, http::header::HeaderMap, http::header::HeaderValue,
};
use actix_web::{
    HttpMessage,
    body::{EitherBody, MessageBody},
    web::Data,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use argon2::password_hash::{SaltString, rand_core};
use argon2::{Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use secrecy::{ExposeSecret, SecretBox};
use sqlx::{PgPool, Row};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Credentials {
    pub username: String,
    pub password: SecretBox<String>,
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretBox<String>)>, anyhow::Error> {
    let result = sqlx::query("SELECT id, password FROM profile WHERE username=$1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve for validatation of auth credentials.")?
        .map(|r| {
            (
                r.get::<Uuid, _>("id"),
                SecretBox::new(Box::new(r.get::<String, _>("password"))),
            )
        });

    Ok(result)
}

#[tracing::instrument(name = "verify password", skip(expected_password, password))]
fn verify_password(
    expected_password: SecretBox<String>,
    password: SecretBox<String>,
) -> Result<(), AuthError> {
    let expected_password = PasswordHash::new(expected_password.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let mut profile_id = None;
    let mut expected_password = SecretBox::new(Box::new(
        "$argon2id$v=19$m=15000,t=2,p=1$h1UJKS5nfDpeNWSscpDd6g$Hm5+wPVIJo5N+Rt+PUlHLhk88e5EHYdb7lRUKCWiW8s".to_string(),
    ));
    if let Some((stored_profile_id, stored_expected_password)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(AuthError::UnexpectedError)?
    {
        profile_id = Some(stored_profile_id);
        expected_password = stored_expected_password;
    };

    spawn_blocking_with_tracing(move || verify_password(expected_password, credentials.password))
        .await
        .context("Failed to spawn blocking task")
        .map_err(AuthError::UnexpectedError)??;

    profile_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username. ")))
}

#[tracing::instrument(name = "Read request access token", skip(headers))]
pub fn read_request_access_token(headers: &HeaderMap) -> Result<String, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The `Authorization` header is missing")?
        .to_str()
        .context("The `Authorization` header was not a valid UTF8 string.")?;

    let access_token = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not `Basic`.")?;

    Ok(access_token.to_string())
}

#[tracing::instrument(name = "Update Password", skip(password, pool))]
pub async fn update_password(
    profile_id: Uuid,
    password: String,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password = spawn_blocking_with_tracing(move || compute_password(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query("UPDATE profile SET password = $1 WHERE id = $2")
        .bind(password)
        .bind(profile_id)
        .execute(pool)
        .await
        .context("Failed to change user's password in the db")?;

    Ok(())
}

pub fn compute_password(password: String) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut rand_core::OsRng);
    let password = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.as_bytes(), &salt)
    .unwrap()
    .to_string();

    Ok(password)
}

#[tracing::instrument(name = "Anonymous Check", skip(req, next))]
pub async fn reject_anonymous_users(
    secret: Data<SecretKey>,
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<EitherBody<impl MessageBody>>, actix_web::Error> {
    let default_www = HeaderValue::from_static("Basic realm=\"task-service\"");
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match (
        session.get_profile_id().map_err(e500)?,
        validate_access_token(&req, &secret.into_inner().0),
    ) {
        (Some(profile_id), Ok(_)) | (Some(profile_id), Err(_)) | (None, Ok(profile_id)) => {
            req.extensions_mut().insert(ProfileId(profile_id));
            let mut res = next.call(req).await?;

            // Adding default header
            let headers = res.headers_mut();
            headers.insert(header::WWW_AUTHENTICATE, default_www);

            Ok(res.map_body(|_, body| EitherBody::left(body)))
        }

        (None, Err(_)) => {
            let message = "You are not logged in. Please log in...";
            let response = HttpResponse::Unauthorized()
                .append_header((header::WWW_AUTHENTICATE, default_www))
                .json(StdResponse { message });

            tracing::warn!(message);

            FlashMessage::error(message).send();

            let res = req.into_response(response);
            Ok(res.map_body(|_, body| EitherBody::right(body)))
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
}

pub fn create_token(
    profile_id: Uuid,
    expiry: u64,
    secret_key: &str,
) -> Result<String, anyhow::Error> {
    let exp = (SystemTime::now() + Duration::from_secs(expiry * 60))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: profile_id,
        exp,
    };

    let jwt = encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(secret_key.as_ref()),
    )?;

    Ok(jwt)
}

pub fn validate_access_token(
    req: &ServiceRequest,
    secret_key: &str,
) -> Result<Uuid, anyhow::Error> {
    let access_token = read_request_access_token(req.headers())?;

    let token_data = decode::<Claims>(
        &access_token,
        &DecodingKey::from_secret(secret_key.as_ref()),
        &Validation::new(Algorithm::HS512),
    )?;
    Ok(token_data.claims.sub)
}
