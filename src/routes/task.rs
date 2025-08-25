use crate::domain::email::ProfileEmail;
use crate::email_client::EmailClient;
use crate::error::task::TaskError;
use crate::model::task::Task;
use crate::model::task::{TaskState, TaskUpdate};
use crate::repository::pgdb;
use crate::telemetry::spawn_blocking_with_tracing;
use actix_web::http::header::HeaderMap;
use actix_web::{HttpRequest, HttpResponse};
use actix_web::{
    get, post, put,
    web::{Data, Json, Path},
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, SecretBox};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

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

struct Credentials {
    username: String,
    password: SecretBox<String>,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The `Authorization` header is missing")?
        .to_str()
        .context("The `Authorization` header was not a valid UTF8 string.")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not `Basic`.")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode `Basic` credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' authorization"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' authorization"))?
        .to_string();

    Ok(Credentials {
        username,
        password: SecretBox::new(Box::new(password)),
    })
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretBox<String>)>, anyhow::Error> {
    let result = sqlx::query("SELECT id, password FROM profile WHERE username=$1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve for validatation of auth credentials.")?
        .map(|r| {
            (
                r.get::<Uuid, _>("id"),
                SecretBox::new(Box::new(r.get::<String, _>("password"))),
            )
        });

    Ok(result)
}

fn verify_password(
    expected_password: SecretBox<String>,
    password: SecretBox<String>,
) -> Result<(), TaskError> {
    let expected_password = PasswordHash::new(expected_password.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(TaskError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password)
        .context("Invalid password")
        .map_err(TaskError::AuthError)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<Uuid, TaskError> {
    let mut profile_id = None;
    let mut expected_password = SecretBox::new(Box::new(
        "$argon2id$v=19$m=15000,t=2,p=1$h1UJKS5nfDpeNWSscpDd6g$Hm5+wPVIJo5N+Rt+PUlHLhk88e5EHYdb7lRUKCWiW8s".to_string(),
    ));
    if let Some((stored_profile_id, stored_expected_password)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(TaskError::UnexpectedError)?
    {
        profile_id = Some(stored_profile_id);
        expected_password = stored_expected_password;
    };

    spawn_blocking_with_tracing(move || verify_password(expected_password, credentials.password))
        .await
        .context("Failed to spawn blocking task")
        .map_err(TaskError::UnexpectedError)??;

    profile_id.ok_or_else(|| TaskError::AuthError(anyhow::anyhow!("Unknown username. ")))
}

#[tracing::instrument(name = "Creating a new task", 
skip(task_request, pool, email_client, request),
fields(task_type=%task_request.task_type,
username=tracing::field::Empty, profile_id=tracing::field::Empty)
)]
#[utoipa::path(
    post,
    path="/task",
    request_body=TaskCreateRequest,
    responses((status=201, description="Task created successfuly"))
)]
#[post("/task")]
pub async fn submit_task(
    pool: Data<PgPool>,
    task_request: Json<TaskCreateRequest>,
    email_client: Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, TaskError> {
    let credentials = basic_authentication(request.headers()).map_err(TaskError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let profile_id = validate_credentials(credentials, &pool).await?;
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

    Ok(HttpResponse::Ok().body("Task successfully created"))
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
