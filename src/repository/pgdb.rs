use crate::model::profile::{Profile, ProfileResponse, ProfileUpdate};
use crate::model::task::{Task, TaskUpdate};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

pub async fn db_create_task(pool: &PgPool, task: Task) -> Result<String, sqlx::Error> {
    let mut tx = pool.begin().await.unwrap();
    let result = sqlx::query("INSERT INTO task(profile_id, task_uuid, task_type, state, source_file, result_file) VALUES($1, $2, $3, $4, $5, $6) RETURNING task_uuid")
        .bind(task.profile_id)
        .bind(task.task_uuid)
        .bind(task.task_type)
        .bind(task.state.to_string())
        .bind(task.source_file)
        .bind(task.result_file)
        .fetch_one(&mut *tx).await;

    match result {
        Ok(row) => {
            let task_uuid: String = row.get("task_uuid");
            tx.commit().await?;
            Ok(task_uuid)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

pub async fn db_update_task(pool: &PgPool, task_update: TaskUpdate) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await.unwrap();
    let mut builder = QueryBuilder::new("UPDATE task SET ");
    let mut separated = builder.separated(", ");

    if let Some(pid) = task_update.profile_id {
        separated.push("profile_id = ").push_bind(pid);
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
        .push(" WHERE task_uuid = ")
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

pub async fn db_get_task(pool: &PgPool, task_id: &str) -> Result<Task, sqlx::Error> {
    let tokens: Vec<String> = task_id.split("_").map(String::from).collect();

    let mut tx = pool.begin().await.unwrap();
    let result =
            sqlx::query_as::<_, Task>("SELECT profile_id, task_uuid, task_type, state, source_file, result_file FROM task WHERE task_uuid= '$1'")
                .bind(tokens[1].clone())
                .fetch_one(&mut *tx).await;

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
        "INSERT INTO profile(id, first_name, last_name, email, status) VALUES($1, $2, $3, $4, $5)",
    )
    .bind(profile.id)
    .bind(profile.first_name.as_ref())
    .bind(profile.last_name.as_ref())
    .bind(profile.email.as_ref())
    .bind("pending_confirmation")
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
