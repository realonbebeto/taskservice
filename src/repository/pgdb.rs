use crate::model::task::Task;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};

#[allow(unused)]
pub struct PGDBRepository {
    pool: Pool<Postgres>,
    table: String,
}

#[allow(unused)]
pub struct PGDBError(sqlx::Error);

impl From<sqlx::Error> for PGDBError {
    fn from(value: sqlx::Error) -> Self {
        PGDBError(value)
    }
}

impl PGDBRepository {
    pub async fn init(table_name: String) -> Result<PGDBRepository, Box<dyn std::error::Error>> {
        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect(
                "postgres://postgres:kuCsggnIu5OQZxJQ@db.siuodfcdskapcblitaco.supabase.co/postgres",
            )
            .await?;

        Ok(PGDBRepository {
            pool,
            table: table_name,
        })
    }

    pub async fn put_task(&self, task: Task) -> Result<String, PGDBError> {
        let mut tx = self.pool.begin().await.unwrap();
        let result = sqlx::query("INSERT INTO (profile_id, task_uuid, task_type, state, source_file, result_file) VALUES($1, $2, $3, $4, $5, $6) RETURNING task_uuid")
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
                Err(PGDBError(e))
            }
        }
    }

    pub async fn get_task(&self, task_id: String) -> Option<Task> {
        let mut tx = self.pool.begin().await.unwrap();
        let result =
            sqlx::query_as::<_, Task>("SELECT profile_id, task_uuid, task_type, state, source_file, result_file x FROM task WHERE task_uuid= '$1'")
                .bind(task_id)
                .fetch_one(&mut *tx).await;

        match result {
            Ok(tsk) => Some(tsk),
            Err(e) => {
                eprint!("{e}");
                None
            }
        }
    }
}
