use crate::model::profile::{Profile, ProfileUpdate};
use crate::model::task::{Task, TaskUpdate};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, QueryBuilder, Row};

#[allow(unused)]
pub struct PGDBRepository {
    pool: Pool<Postgres>,
}

#[allow(unused)]
#[derive(Debug)]
pub struct PGDBError(sqlx::Error);

impl From<sqlx::Error> for PGDBError {
    fn from(value: sqlx::Error) -> Self {
        PGDBError(value)
    }
}

#[allow(unused)]
impl PGDBRepository {
    pub async fn init() -> Result<PGDBRepository, Box<dyn std::error::Error>> {
        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect(
                "postgres://postgres:kuCsggnIu5OQZxJQ@db.siuodfcdskapcblitaco.supabase.co/postgres",
            )
            .await?;

        Ok(PGDBRepository { pool })
    }

    async fn task_exists(&self, id: &str) -> bool {
        let mut conn = self.pool.acquire().await.unwrap();
        let result = sqlx::query("SELECT id FROM task WHERE id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await;

        match result {
            Ok(_) => true,
            Err(e) => {
                eprintln!("{e}");
                false
            }
        }
    }

    async fn profile_exists(&self, id: &str) -> bool {
        let mut conn = self.pool.acquire().await.unwrap();
        let result = sqlx::query("SELECT id FROM profile WHERE id = $1")
            .bind(id)
            .execute(&mut *conn)
            .await;

        match result {
            Ok(_) => true,
            Err(e) => {
                eprintln!("{e}");
                false
            }
        }
    }

    pub async fn create_task(&self, task: Task) -> Result<String, PGDBError> {
        let mut tx = self.pool.begin().await.unwrap();
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
                Err(PGDBError(e))
            }
        }
    }

    pub async fn update_task(&self, task_update: TaskUpdate) -> Result<(), PGDBError> {
        let mut tx = self.pool.begin().await.unwrap();
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
                Err(PGDBError(e))
            }
        }
    }

    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let tokens: Vec<String> = task_id.split("_").map(|x| String::from(x)).collect();

        let mut tx = self.pool.begin().await.unwrap();
        let result =
            sqlx::query_as::<_, Task>("SELECT profile_id, task_uuid, task_type, state, source_file, result_file FROM task WHERE task_uuid= '$1'")
                .bind(tokens[1].clone())
                .fetch_one(&mut *tx).await;

        match result {
            Ok(tsk) => Some(tsk),
            Err(e) => {
                eprintln!("{e}");
                None
            }
        }
    }

    pub async fn create_profile(&self, profile: &Profile) -> Result<String, PGDBError> {
        if self.profile_exists(&profile.id).await {
            return Err(PGDBError(sqlx::Error::InvalidArgument(
                "Profile already exists".into(),
            )));
        }
        let mut tx = self.pool.begin().await.unwrap();
        let result = sqlx::query(
            "INSERT INTO profile(id, first_name, last_name) VALUES($1, $2, $3) RETURNING id",
        )
        .bind(profile.id.clone())
        .bind(profile.first_name.clone())
        .bind(profile.last_name.clone())
        .fetch_one(&mut *tx)
        .await;

        match result {
            Ok(row) => {
                let pid = row.get("id");
                tx.commit().await?;
                Ok(pid)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(PGDBError(e))
            }
        }
    }

    pub async fn get_profile(&self, id: &str) -> Option<Profile> {
        let mut tx = self.pool.begin().await.unwrap();
        let result = sqlx::query_as::<_, Profile>("SELECT id, first_name, last_name WHERE id='$1'")
            .bind(id)
            .fetch_one(&mut *tx)
            .await;

        match result {
            Ok(prf) => Some(prf),
            Err(e) => {
                eprintln!("{e}");
                None
            }
        }
    }

    pub async fn delete_profile(&self, id: &str) -> Result<String, PGDBError> {
        if !self.profile_exists(id).await {
            return Err(PGDBError(sqlx::Error::RowNotFound));
        }

        let mut tx = self.pool.begin().await.unwrap();
        let result = sqlx::query("DELETE FROM profile WHERE id = '$1' RETURNING id")
            .bind(id)
            .fetch_one(&mut *tx)
            .await;

        match result {
            Ok(row) => {
                let pid = row.get("id");
                tx.commit().await?;
                Ok(pid)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(PGDBError(e))
            }
        }
    }

    pub async fn update_profile(&self, profile_update: &ProfileUpdate) -> Result<(), PGDBError> {
        let mut tx = self.pool.begin().await.unwrap();
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
                Err(PGDBError(e))
            }
        }
    }
}
