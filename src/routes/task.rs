use crate::domain::id::ProfileId;
use crate::error::authentication::StdResponse;
use crate::error::task::TaskError;
use crate::idempotency::{IdempotencyKey, NextAction, save_response, try_idem_processing};
use crate::model::task::Task;
use crate::model::task::{TaskState, TaskUpdate};
use crate::repository::pgdb;
use crate::util::{e400, e500};
use actix_web::{
    HttpResponse, get, put,
    web::{Data, Json, Path, ReqData},
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct TaskIdentifier {
    task_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCompletionRequest {
    result_file: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCreateRequest {
    task_type: String,
    source_file: String,
    idempotency_key: String,
}

async fn state_transition(
    pool: Data<PgPool>,
    task_id: Uuid,
    new_state: TaskState,
    result_file: Option<String>,
) -> Result<TaskIdentifier, TaskError> {
    let task = pgdb::db_get_task(pool.get_ref(), task_id)
        .await
        .context("Failed to fetch associated task for task state transition")?;

    task.can_transition_to(&new_state)?;

    let task_update = TaskUpdate::new(task_id, None, None, Some(new_state), None, result_file);

    pgdb::db_update_task(pool.get_ref(), task_update)
        .await
        .context("Failed to update task")?;

    Ok(TaskIdentifier { task_id })
}
#[utoipa::path(get, path = "/task/{task_global_id}",
params(("task_gloabal_id"= String, Path, description="Global Id")),
responses((status=200, body=Task, description="Task by ID"), (status=404, description="Task not found"),))]
#[get("/task/{task_global_id}")]
pub async fn get_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<HttpResponse, TaskError> {
    pgdb::db_get_task(pool.get_ref(), task_identifier.into_inner().task_id)
        .await
        .context("Failed to fetch associated task")?;
    // TODO
    Ok(HttpResponse::Ok().body("Successful"))
}

#[tracing::instrument(name = "Creating a new task", 
skip(task_request, pool),
fields(task_type=%task_request.task_type,
username=tracing::field::Empty, profile_id=tracing::field::Empty)
)]
#[utoipa::path(
    post,
    path="/admin/task",
    request_body=TaskCreateRequest,
    responses((status=201, description="Task created successfuly"))
)]
pub async fn create_task(
    pool: Data<PgPool>,
    task_request: Json<TaskCreateRequest>,
    profile_id: ReqData<ProfileId>,
) -> Result<HttpResponse, actix_web::Error> {
    let profile_id = profile_id.0;

    let TaskCreateRequest {
        task_type,
        source_file,
        idempotency_key,
    } = task_request.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let cookiex = FlashMessage::success("The task has been created and sent out");

    let mut transaction = match try_idem_processing(&pool, &idempotency_key, profile_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(sr) => {
            cookiex.send();
            return Ok(sr);
        }
    };

    let task = Task::new(profile_id, task_type.clone(), source_file.clone());

    pgdb::db_create_task(&mut transaction, &task)
        .await
        .context("Failed to create new task")
        .map_err(e500)?;

    pgdb::enqueue_delivery_tasks(&mut transaction, &task)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = HttpResponse::Ok().json(StdResponse {
        message: "Task successfully created",
    });

    let response = save_response(transaction, &idempotency_key, profile_id, response)
        .await
        .map_err(e500)?;

    cookiex.send();

    Ok(response)
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
) -> Result<HttpResponse, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_id,
        TaskState::InProgress,
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().body("Successful"))
}

#[utoipa::path(put, path="/task/{task_global_id}",
params(("task_global_id" = String, Path, description="Global Id")),
request_body= TaskIdentifier,
responses((status=200, description="Task pause successful"), (status=424, description="Task pause unsuccessful")))]
#[put("/task/{task_global_id}/pause")]
pub async fn pause_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<HttpResponse, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_id,
        TaskState::Paused,
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().body("Successful"))
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
) -> Result<HttpResponse, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_id,
        TaskState::Completed,
        Some(complete_request.result_file.clone()),
    )
    .await?;

    Ok(HttpResponse::Ok().body("Successful"))
}

#[utoipa::path(put, path="/task/{task_global_id}/fail",
params(("task_global_id"=String, Path, description="Global Id")),
responses((status=200, description="Task fail successful"), (status=424, description="Task fail unsuccessful")))]
#[put("/task/{task_global_id}/fail")]
pub async fn fail_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<HttpResponse, TaskError> {
    state_transition(
        pool,
        task_identifier.into_inner().task_id,
        TaskState::Failed,
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().body("Successful"))
}
