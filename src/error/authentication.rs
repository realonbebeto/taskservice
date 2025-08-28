use actix_web::{HttpResponse, ResponseError, http::StatusCode};

use crate::error::common::error_chain_fmt;
use actix_web_flash_messages::FlashMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Serialize, Deserialize)]
pub struct StdResponse<'a> {
    pub message: &'a str,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("It's not you, it's us")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        FlashMessage::error(self.to_string()).send();
        HttpResponse::build(self.status_code())
            // .cookie(Cookie::new("_flash", self.to_string()))
            .json(StdResponse {
                message: &self.to_string(),
            })
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Cookie::build("session_id", "abcdef123456")
//         .path("/")
