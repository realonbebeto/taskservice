use crate::error::authentication::AuthError;
use crate::telemetry::spawn_blocking_with_tracing;
use actix_web::http::header::HeaderMap;
use anyhow::Context;
use argon2::password_hash::{SaltString, rand_core};
use argon2::{Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::{PgPool, Row};
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

#[tracing::instrument(name = "Validate credentials", skip(headers))]
pub fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The `Authorization` header is missing")?
        .to_str()
        .context("The `Authorization` header was not a valid UTF8 string.")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not `Basic`.")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode `Basic` credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' authorization"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' authorization"))?
        .to_string();

    Ok(Credentials {
        username,
        password: SecretBox::new(Box::new(password)),
    })
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
