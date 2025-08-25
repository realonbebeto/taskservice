use crate::error::common::error_chain_fmt;
use actix_web::http::header::HeaderValue;
use actix_web::http::{StatusCode, header};
use actix_web::{HttpResponse, ResponseError};

#[derive(thiserror::Error)]
pub enum TaskError {
    // TaskNotFound,
    // TaskUpdateFailure,
    // TaskCreationFailure,
    // BadTaskRequest,
    // DatabaseError(sqlx::Error),
    #[error("{0}")]
    TransitionError(String),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for TaskError {
    fn error_response(&self) -> HttpResponse {
        match self {
            TaskError::TransitionError(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            TaskError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="task-service""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            TaskError::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

// impl ResponseError for TaskError {
//     fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
//         HttpResponse::build(self.status_code())
//             .insert_header(ContentType::json())
//             .body(self.to_string())
//     }c
//     fn status_code(&self) -> StatusCode {
//         match self {
//             TaskError::TaskNotFound => StatusCode::NOT_FOUND,
//             TaskError::TaskUpdateFailure => StatusCode::FAILED_DEPENDENCY,
//             TaskError::TaskCreationFailure => StatusCode::FAILED_DEPENDENCY,
//             TaskError::BadTaskRequest => StatusCode::BAD_REQUEST,
//         }
//     }
// }
