use crate::model::task::Task;
use crate::model::task::{TaskState, TaskUpdate};
use crate::repository::pgdb;
use actix_web::{
    HttpResponse,
    error::ResponseError,
    get,
    http::{StatusCode, header::ContentType},
    post, put,
    web::{Data, Json, Path},
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct TaskIdentifier {
    task_global_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCompletionRequest {
    result_file: String,
}

#[derive(Debug, Display)]
pub enum TaskError {
    TaskNotFound,
    TaskUpdateFailure,
    TaskCreationFailure,
    BadTaskRequest,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCreateRequest {
    profile_id: String,
    task_type: String,
    source_file: String,
}

impl ResponseError for TaskError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match self {
            TaskError::TaskNotFound => StatusCode::NOT_FOUND,
            TaskError::TaskUpdateFailure => StatusCode::FAILED_DEPENDENCY,
            TaskError::TaskCreationFailure => StatusCode::FAILED_DEPENDENCY,
            TaskError::BadTaskRequest => StatusCode::BAD_REQUEST,
        }
    }
}

async fn state_transition(
    pool: Data<PgPool>,
    task_global_id: String,
    new_state: TaskState,
    result_file: Option<String>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    let task = match pgdb::db_get_task(pool.get_ref(), &task_global_id).await {
        Some(task) => task,
        None => return Err(TaskError::TaskNotFound),
    };

    if !task.can_transition_to(&new_state) {
        return Err(TaskError::BadTaskRequest);
    };

    let tokens: Vec<String> = task_global_id.split("_").map(String::from).collect();

    let task_update = TaskUpdate::new(
        tokens[1].clone(),
        None,
        None,
        Some(new_state),
        None,
        result_file,
    );

    let task_identifier = task.get_global_id();
    match pgdb::db_update_task(pool.get_ref(), task_update).await {
        Ok(_) => Ok(Json(TaskIdentifier {
            task_global_id: task_identifier,
        })),
        Err(_) => Err(TaskError::TaskUpdateFailure),
    }
}
#[utoipa::path(get, path = "/task/{task_global_id}",
params(("task_gloabal_id"= String, Path, description="Global Id")),
responses((status=200, body=Task, description="Task by ID"), (status=404, description="Task not found"),))]
#[get("/task/{task_global_id}")]
pub async fn get_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<Json<Task>, TaskError> {
    let tsk = pgdb::db_get_task(pool.get_ref(), &task_identifier.into_inner().task_global_id).await;
    match tsk {
        Some(tsk) => Ok(Json(tsk)),
        None => Err(TaskError::TaskNotFound),
    }
}

#[utoipa::path(
    post,
    path="/task",
    request_body=TaskCreateRequest,
    responses((status=201, description="Task submitted successfuly"))
)]
#[post("/task")]
pub async fn submit_task(
    pool: Data<PgPool>,
    request: Json<TaskCreateRequest>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    let task = Task::new(
        request.profile_id.clone(),
        request.task_type.clone(),
        request.source_file.clone(),
    );

    let task_identifier = task.get_global_id();
    match pgdb::db_create_task(pool.get_ref(), task).await {
        Ok(_) => Ok(Json(TaskIdentifier {
            task_global_id: task_identifier,
        })),
        Err(_) => Err(TaskError::TaskCreationFailure),
    }
}

#[utoipa::path(put, path="/task/{task_global_id}",
params(("task_global_id" = String, Path, description="Global Id")),
request_body = TaskIdentifier,
responses((status=200, description="Task start successful"), 
            (status=424, description="Task start unsuccessful")))]
#[put("/task/{task_global_id}/start")]
pub async fn start_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_global_id,
        TaskState::InProgress,
        None,
    )
    .await
}

#[utoipa::path(put, path="/task/{task_global_id}",
params(("task_global_id" = String, Path, description="Global Id")),
request_body= TaskIdentifier,
responses((status=200, description="Task pause successful"), (status=424, description="Task pause unsuccessful")))]
#[put("/task/{task_global_id}/pause")]
pub async fn pause_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_global_id,
        TaskState::Paused,
        None,
    )
    .await
}
#[utoipa::path(put, path="/task/{task_global_id}/complete",
params(("task_global_id" = String, Path, description="Global Id")),
request_body=TaskCompletionRequest,
responses((status=200, description="Task completion successful"), (status=424, description="Task completion unsuccessful")))]
#[put("/task/{task_global_id}/complete")]
pub async fn complete_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
    complete_request: Json<TaskCompletionRequest>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_global_id,
        TaskState::Completed,
        Some(complete_request.result_file.clone()),
    )
    .await
}

#[utoipa::path(put, path="/task/{task_global_id}/fail",
params(("task_global_id"=String, Path, description="Global Id")),
responses((status=200, description="Task fail successful"), (status=424, description="Task fail unsuccessful")))]
#[put("/task/{task_global_id}/fail")]
pub async fn fail_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<Json<TaskIdentifier>, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_global_id,
        TaskState::Failed,
        None,
    )
    .await
}
