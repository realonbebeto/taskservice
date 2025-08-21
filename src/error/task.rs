use actix_web::ResponseError;

#[derive(Debug)]
pub enum TaskError {
    TaskNotFound,
    TaskUpdateFailure,
    TaskCreationFailure,
    BadTaskRequest,
    DatabaseError(sqlx::Error),
    TransitionError(String),
}

impl From<sqlx::Error> for TaskError {
    fn from(value: sqlx::Error) -> Self {
        Self::DatabaseError(value)
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to create a new subscriber")
    }
}

impl ResponseError for TaskError {}

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
