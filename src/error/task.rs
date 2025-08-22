use crate::error::common::error_chain_fmt;
use actix_web::ResponseError;
use actix_web::http::StatusCode;

#[derive(thiserror::Error)]
pub enum TaskError {
    // TaskNotFound,
    // TaskUpdateFailure,
    // TaskCreationFailure,
    // BadTaskRequest,
    // DatabaseError(sqlx::Error),
    #[error("{0}")]
    TransitionError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for TaskError {
    fn status_code(&self) -> StatusCode {
        match self {
            TaskError::TransitionError(_) => StatusCode::BAD_REQUEST,
            TaskError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// impl ResponseError for TaskError {
//     fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
//         HttpResponse::build(self.status_code())
//             .insert_header(ContentType::json())
//             .body(self.to_string())
//     }
//     fn status_code(&self) -> StatusCode {
//         match self {
//             TaskError::TaskNotFound => StatusCode::NOT_FOUND,
//             TaskError::TaskUpdateFailure => StatusCode::FAILED_DEPENDENCY,
//             TaskError::TaskCreationFailure => StatusCode::FAILED_DEPENDENCY,
//             TaskError::BadTaskRequest => StatusCode::BAD_REQUEST,
//         }
//     }
// }
