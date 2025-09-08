use actix_web::{HttpRequest, HttpResponse, cookie::Cookie, get, http::header, post, web};

use secrecy::SecretBox;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    authentication::{Credentials, create_token, validate_credentials, validate_refresh_token},
    error::authentication::{AuthError, LoginError, StdResponse},
    startup::{ExpiryTime, SecretKey},
};

use crate::session_state::TypedSession;
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use std::fmt::Write;

#[derive(serde::Deserialize, ToSchema, Debug)]
pub struct LoginData {
    username: String,
    password: String,
}

#[tracing::instrument(name = "Logging In", skip(form, pool, session))]
#[utoipa::path(post, path = "/login", responses((status=200, description="Authentication successful"), (status=401, description="Authentication failed")))]
#[post("/login")]
async fn log_in(
    form: web::Form<LoginData>,
    pool: web::Data<PgPool>,
    secret: web::Data<SecretKey>,
    expiry_time: web::Data<ExpiryTime>,
    session: TypedSession,
) -> Result<HttpResponse, LoginError> {
    let secret = &secret.into_inner().0;
    let credentials = Credentials {
        username: form.0.username,
        password: SecretBox::new(Box::new(form.0.password)),
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(profile_id) => {
            tracing::Span::current().record("profile_id", tracing::field::display(&profile_id));

            session.renew();

            session
                .insert_profile_id(profile_id)
                .map_err(|e| LoginError::UnexpectedError(e.into()))?;

            FlashMessage::info("Authorized").send();
            dbg!("login");

            let access_token = create_token(profile_id, expiry_time.into_inner().0, secret)?;

            let refresh_token = create_token(profile_id, 30 * 24 * 60, secret)?;

            Ok(HttpResponse::Ok()
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", access_token)))
                .cookie(
                    Cookie::build("refresh_token", refresh_token)
                        .http_only(true)
                        .finish(),
                )
                .json(StdResponse {
                    message: "Login Successful",
                }))
        }
        Err(e) => match e {
            AuthError::InvalidCredentials(_) => Err(LoginError::AuthError(e.into())),
            AuthError::UnexpectedError(_) => Err(LoginError::UnexpectedError(e.into())),
        },
    }
}

#[tracing::instrument(name = "Logging In", skip(flash_msgs))]
#[utoipa::path(get, path = "/login", responses((status=200, description="Successful login"), (status=401, description="Authentication failed")))]
#[get("/login")]
async fn log_in_check(flash_msgs: IncomingFlashMessages) -> HttpResponse {
    let mut msg = String::new();

    for m in flash_msgs.iter().filter(|m| m.level() == Level::Error) {
        writeln!(msg, "{}", m.content()).unwrap();
    }

    HttpResponse::Ok().json(StdResponse { message: &msg })
}

#[tracing::instrument(name = "Refresh Token", skip(req))]
#[utoipa::path(get, path = "/refresh-token", responses((status=200, description="Successful refresh"), (status=401, description="Refresh failed")))]
pub async fn refresh_token(
    req: HttpRequest,
    secret: web::Data<SecretKey>,
    expiry_time: web::Data<ExpiryTime>,
) -> Result<HttpResponse, LoginError> {
    let secret = &secret.into_inner().0;

    let refresh_token = req.cookie("refresh_token");

    match refresh_token {
        Some(_) => {
            let profile_id = validate_refresh_token(&req, secret).map_err(LoginError::AuthError)?;
            let access_token = create_token(profile_id, expiry_time.into_inner().0, secret)?;
            let refresh_token = create_token(profile_id, 30 * 24 * 60, secret)?;

            Ok(HttpResponse::Ok()
                .insert_header((header::AUTHORIZATION, format!("Bearer {}", access_token)))
                .cookie(
                    Cookie::build("refresh_token", refresh_token)
                        .http_only(true)
                        .finish(),
                )
                .json(StdResponse {
                    message: "Access token refreshed",
                }))
        }
        None => Err(LoginError::AuthError(anyhow::anyhow!(
            "Unauthorized to refresh token"
        ))),
    }
}
