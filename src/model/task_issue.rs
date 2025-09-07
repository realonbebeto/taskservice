use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct Issue {
    pub task_issue_id: Uuid,
    pub profile_email: String,
}
