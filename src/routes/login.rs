use actix_web::{HttpResponse, get, post, web};

use secrecy::SecretBox;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    authentication::{Credentials, validate_credentials},
    error::authentication::{AuthError, LoginError, LoginResponse},
};

use crate::session_state::TypedSession;
use actix_web_flash_messages::{IncomingFlashMessages, Level};
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
    session: TypedSession,
) -> Result<HttpResponse, LoginError> {
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

            Ok(HttpResponse::Ok().json(LoginResponse {
                message: "login successful".into(),
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

    HttpResponse::Ok().json(LoginResponse { message: msg })
}
