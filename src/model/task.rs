use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum_macros::Display;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::task::TaskError;

#[derive(sqlx::Type)] // only for PostgreSQL to match a type definition
#[derive(Serialize, Deserialize, Display, Debug, Eq, PartialEq, ToSchema)]
pub enum TaskState {
    NotStarted,
    InProgress,
    Completed,
    Paused,
    Failed,
}

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct Task {
    pub profile_id: Uuid,
    pub task_uuid: Uuid,
    pub task_type: String,
    pub state: TaskState,
    pub source_file: String,
    pub result_file: Option<String>,
}

impl Task {
    pub fn new(profile_id: Uuid, task_type: String, source_file: String) -> Task {
        Task {
            profile_id,
            task_uuid: Uuid::new_v4(),
            task_type,
            state: TaskState::NotStarted,
            source_file,
            result_file: None,
        }
    }

    pub fn get_global_id(&self) -> String {
        format!("{}_{}", self.profile_id, self.task_uuid)
    }

    pub fn can_transition_to(&self, state: &TaskState) -> Result<(), TaskError> {
        if self.state == *state {
            return Err(TaskError::TransitionError(format!(
                "{} = {} transition not possible",
                self.state, state
            )));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaskUpdate {
    pub task_uuid: String,
    pub profile_id: Option<String>,
    pub task_type: Option<String>,
    pub state: Option<TaskState>,
    pub source_file: Option<String>,
    pub result_file: Option<String>,
}

impl TaskUpdate {
    pub fn new(
        task_uuid: String,
        profile_id: Option<String>,
        task_type: Option<String>,
        state: Option<TaskState>,
        source_file: Option<String>,
        result_file: Option<String>,
    ) -> TaskUpdate {
        TaskUpdate {
            task_uuid,
            profile_id,
            task_type,
            state,
            source_file,
            result_file,
        }
    }
}
