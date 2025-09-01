use crate::model::profile::{Profile, ProfileResponse, ProfileUpdate};
use crate::model::task::{Task, TaskUpdate};
use anyhow::Context;
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

pub async fn db_create_task(
    tx: &mut Transaction<'_, Postgres>,
    task: &Task,
) -> Result<(), sqlx::Error> {
    // let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO task(reporter_id, id, task_type, state, source_file, result_file) VALUES($1, $2, $3, $4, $5, $6)")
        .bind(task.reporter_id)
        .bind(task.id)
        .bind(task.task_type.clone())
        .bind(&task.state)
        .bind(task.source_file.clone())
        .bind(task.result_file.as_ref())
        .execute(&mut **tx).await?;

    Ok(())
}

pub async fn db_update_task(pool: &PgPool, task_update: TaskUpdate) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await.unwrap();
    let mut builder = QueryBuilder::new("UPDATE task SET ");
    let mut separated = builder.separated(", ");

    if let Some(pid) = task_update.profile_id {
        separated.push("reporter_id = ").push_bind(pid);
    }

    if let Some(task_type) = task_update.task_type {
        separated.push("task_type = ").push_bind(task_type);
    }

    if let Some(state) = task_update.state {
        separated.push("state = ").push_bind(state);
    }

    if let Some(source_file) = task_update.source_file {
        separated.push("state = ").push_bind(source_file);
    }

    if let Some(result_file) = task_update.result_file {
        separated.push("state = ").push_bind(result_file);
    }

    builder
        .push(" WHERE id = ")
        .push_bind(task_update.task_uuid);

    let result = builder.build().execute(&mut *tx).await;

    match result {
        Ok(_) => {
            tx.commit().await?;
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}
#[tracing::instrument(skip_all)]
pub async fn db_get_task(pool: &PgPool, task_id: Uuid) -> Result<Task, sqlx::Error> {
    let result = sqlx::query_as::<_, Task>(
        "SELECT reporter_id, id, task_type, state, source_file, result_file FROM task WHERE id= $1",
    )
    .bind(task_id)
    .fetch_one(pool)
    .await;

    match result {
        Ok(tsk) => Ok(tsk),
        Err(e) => Err(e),
    }
}

#[tracing::instrument("Saving new profile details in the database", skip(tx, profile))]
pub async fn db_create_profile(
    tx: &mut Transaction<'_, Postgres>,
    profile: &Profile,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO profile(id, first_name, last_name, email, status, username, password) VALUES($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(profile.id)
    .bind(profile.first_name.as_ref())
    .bind(profile.last_name.as_ref())
    .bind(profile.email.as_ref())
    .bind("pending_confirmation")
    .bind(profile.username.as_ref())
    .bind(profile.password.phash_as_ref())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn db_get_profile(pool: &PgPool, id: &Uuid) -> Result<ProfileResponse, sqlx::Error> {
    let result = sqlx::query_as::<_, ProfileResponse>(
        "SELECT id, first_name, last_name, email FROM profile WHERE id=$1",
    )
    .bind(id)
    .persistent(false)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub async fn delete_profile(pool: &PgPool, id: &Uuid) -> Result<(), sqlx::Error> {
    let profile = db_get_profile(pool, id).await;

    match profile {
        Ok(_) => {
            let mut tx = pool.begin().await.unwrap();
            let result = sqlx::query("DELETE FROM profile WHERE id = '$1' RETURNING id")
                .bind(id)
                .fetch_one(&mut *tx)
                .await;

            match result {
                Ok(_) => {
                    tx.commit().await?;
                    Ok(())
                }
                Err(e) => {
                    tx.rollback().await?;
                    Err(e)
                }
            }
        }
        Err(e) => Err(e),
    }
}

pub async fn db_update_profile(
    pool: &PgPool,
    profile_update: &ProfileUpdate,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await.unwrap();
    let mut builder = QueryBuilder::new("UPDATE profile SET ");
    let mut separated = builder.separated(", ");

    if let Some(f_name) = profile_update.first_name.clone() {
        separated.push("first_name = ").push_bind(f_name);
    }

    if let Some(l_name) = profile_update.last_name.clone() {
        separated.push("last_name = ").push_bind(l_name);
    }

    builder
        .push(" WHERE id = ")
        .push_bind(profile_update.id.clone());

    let result = builder.build().execute(&mut *tx).await;

    match result {
        Ok(_) => {
            tx.commit().await?;
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

#[tracing::instrument(name = "Get Username", skip(pool))]
pub async fn get_username(profile_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query("SELECT username FROM profile WHERE id = $1")
        .bind(profile_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform query to retrieve username")?;

    Ok(row.get("username"))
}

#[tracing::instrument(skip_all)]
pub async fn enqueue_delivery_tasks(
    tx: &mut Transaction<'_, Postgres>,
    task: &Task,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO issue_delivery_queue (task_issue_id, profile_email)
                SELECT $1, email FROM profile WHERE status = 'confirmed'",
    )
    .bind(task.id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
