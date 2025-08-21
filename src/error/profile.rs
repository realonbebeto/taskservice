use actix_web::{ResponseError, http::StatusCode};

use crate::error::common::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum ProfileError {
    #[error("{0}")]
    ValidationError(String),
    // #[error("Failed to acquire a Postgres connection from the pool")]
    // PoolError(#[source] sqlx::Error),
    // #[error("Failed to insert new profile in the database")]
    // InsertProfileError(#[source] sqlx::Error),
    // #[error("Failed to commit SQL transaction to store new profile")]
    // TransactionCommitError(#[source] sqlx::Error),
    // #[error("Failed to confirm a new profile")]
    // ConfirmError(#[source] sqlx::Error),
    // #[error("Associated profile token not found")]
    // TokenNotFound(#[source] sqlx::Error),
    // #[error("Associated profile not found")]
    // ProfileNotFound(#[source] sqlx::Error),
    // #[error("Failed to store the confirmation token for a new profile.")]
    // StoreTokenError(#[from] StoreTokenError),
    // #[error("Failed to send a confirmation email.")]
    // SendEmailError(#[from] reqwest::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for ProfileError {
    // fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
    //     HttpResponse::build(self.status_code())
    //         .insert_header(ContentType::json())
    //         .body(self.to_string())
    // }
    fn status_code(&self) -> StatusCode {
        match self {
            ProfileError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ProfileError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Debug for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
