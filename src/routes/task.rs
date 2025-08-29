use crate::domain::email::ProfileEmail;
use crate::email_client::EmailClient;
use crate::error::authentication::StdResponse;
use crate::error::task::TaskError;
use crate::model::task::Task;
use crate::model::task::{TaskState, TaskUpdate};
use crate::repository::pgdb;
use crate::session_state::TypedSession;
use actix_web::HttpResponse;
use actix_web::{
    get, put,
    web::{Data, Json, Path},
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct TaskIdentifier {
    task_global_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCompletionRequest {
    result_file: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TaskCreateRequest {
    task_type: String,
    source_file: String,
}

async fn state_transition(
    pool: Data<PgPool>,
    task_global_id: String,
    new_state: TaskState,
    result_file: Option<String>,
) -> Result<TaskIdentifier, TaskError> {
    let task = pgdb::db_get_task(pool.get_ref(), &task_global_id)
        .await
        .context("Failed to fetch associated task for task state transition")?;

    task.can_transition_to(&new_state)?;

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
    pgdb::db_update_task(pool.get_ref(), task_update)
        .await
        .context("Failed to update task")?;

    Ok(TaskIdentifier {
        task_global_id: task_identifier,
    })
}
#[utoipa::path(get, path = "/task/{task_global_id}",
params(("task_gloabal_id"= String, Path, description="Global Id")),
responses((status=200, body=Task, description="Task by ID"), (status=404, description="Task not found"),))]
#[get("/task/{task_global_id}")]
pub async fn get_task(
    pool: Data<PgPool>,
    task_identifier: Path<TaskIdentifier>,
) -> Result<HttpResponse, TaskError> {
    pgdb::db_get_task(pool.get_ref(), &task_identifier.into_inner().task_global_id)
        .await
        .context("Failed to fetch associated task")?;
    // TODO
    Ok(HttpResponse::Ok().body("Successful"))
}

struct ConfirmedProfile {
    email: ProfileEmail,
}

#[tracing::instrument(name = "Get confirmed profiles", skip(pool))]
async fn get_confirmed_profiles(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedProfile, anyhow::Error>>, sqlx::Error> {
    let rows = sqlx::query("SELECT email FROM profile WHERE status= 'confirmed'")
        .fetch_all(pool)
        .await?;

    let confirmed_profiles = rows
        .into_iter()
        .map(|r| match ProfileEmail::parse(r.get("email")) {
            Ok(email) => Ok(ConfirmedProfile { email }),
            Err(e) => Err(anyhow::anyhow!(e)),
        })
        .collect();

    Ok(confirmed_profiles)
}

#[tracing::instrument(name = "Creating a new task", 
skip(task_request, pool, email_client, session),
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
    email_client: Data<EmailClient>,
    session: TypedSession,
) -> Result<HttpResponse, TaskError> {
    dbg!(1);
    let profile_id = match session.get_profile_id().map_err(|e| {
        let e = e.to_string();
        tracing::error!(e);
        TaskError::UnexpectedError(anyhow::anyhow!("It is us not you, its us"))
    })? {
        None => {
            return Ok(HttpResponse::Unauthorized().json(StdResponse {
                message: "Not allowed",
            }));
        }
        Some(profile_id) => profile_id,
    };

    tracing::Span::current().record("profile_id", tracing::field::display(&profile_id));

    let task = Task::new(
        profile_id,
        task_request.task_type.clone(),
        task_request.source_file.clone(),
    );

    let profiles = get_confirmed_profiles(&pool)
        .await
        .context("Failed to fetch confirmed profiles")?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed Failed to acquire a Postgres connection from the pool")?;
    pgdb::db_create_task(&mut transaction, &task)
        .await
        .context("Failed to create new task")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store new task")?;

    for profile in profiles {
        match profile {
            Ok(profile) => {
                email_client
                    .send_email(
                        &profile.email,
                        "New Task",
                        &task.task_type,
                        &task.source_file,
                    )
                    .await
                    .with_context(|| format!("Failed to send task issue to {}", profile.email))?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a coonfirmed profile. Their stored contact details are invalid");
            }
        }
    }

    Ok(HttpResponse::Ok().json(StdResponse {
        message: "Task successfully created",
    }))
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
        task_identifier.into_inner().task_global_id,
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
        task_identifier.into_inner().task_global_id,
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
        task_identifier.into_inner().task_global_id,
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
        task_identifier.into_inner().task_global_id,
        TaskState::Failed,
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().body("Successful"))
}
