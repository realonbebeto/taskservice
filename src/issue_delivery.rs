use std::time::Duration;

use crate::repository::pgdb;
use crate::{configuration::Settings, startup::get_connection_pool};
use crate::{domain::email::ProfileEmail, email_client::EmailClient};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
use tracing::{Span, field::display};
use uuid::Uuid;

type PgTx = Transaction<'static, Postgres>;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(pool: &PgPool) -> Result<Option<(PgTx, Uuid, String)>, anyhow::Error> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "SELECT task_issue_id, profile_email FROM issue_delivery_queue
            FOR UPDATE SKIP LOCKED
            LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(r) = result {
        Ok(Some((tx, r.get("task_issue_id"), r.get("profile_email"))))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(mut tx: PgTx, issue_id: Uuid, email: &str) -> Result<(), anyhow::Error> {
    sqlx::query(
        "DELETE FROM issue_delivery_queue
    WHERE task_issue_id= $1
    AND profile_email=$2",
    )
    .bind(issue_id)
    .bind(email)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(task_issue_id=tracing::field::Empty, profile_email=tracing::field::Empty))]
pub async fn try_execute_delivery(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(pool).await?;

    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (tx, issue_id, email) = task.unwrap();

    Span::current()
        .record("task_issue_id", display(issue_id))
        .record("profile_email", display(&email));

    match ProfileEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = pgdb::db_get_task(pool, issue_id).await?;

            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.task_type,
                    &issue.source_file,
                    &issue.source_file,
                )
                .await
            {
                tracing::error!(error.cause_chain = ?e, error.message=%e, "Failed to deliver issue to a confirmed profile. Skipping");
            }
        }
        Err(e) => {
            tracing::error!(error.cause_chain = ?e, error.message=%e, "Skipping a confirmed profile. Their stored contact details are invalid");
        }
    }

    delete_task(tx, issue_id, &email).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

async fn delivery_worker_loop(
    pool: PgPool,
    email_client: EmailClient,
) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_delivery(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_delivery_worker_until_stopped(
    configuration: Arc<Settings>,
) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);

    let email_client = configuration.email_client.client();

    delivery_worker_loop(connection_pool, email_client).await
}
