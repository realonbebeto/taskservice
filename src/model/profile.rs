use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[allow(unused)]
#[derive(Serialize, Deserialize, FromRow, Debug, ToSchema)]
pub struct Profile {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
}

impl Profile {
    pub fn new(first_name: &str, last_name: &str) -> Profile {
        Profile {
            id: Uuid::new_v4().to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
        }
    }
}

#[allow(unused)]
#[derive(Serialize, Deserialize, FromRow, Debug, ToSchema)]
pub struct ProfileUpdate {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

impl ProfileUpdate {
    pub fn new(id: &str, first_name: Option<&str>, last_name: Option<&str>) -> ProfileUpdate {
        let f_name = first_name.map(|v| v.to_string());

        let l_name = last_name.map(|v| v.to_string());

        ProfileUpdate {
            id: id.to_string(),
            first_name: f_name,
            last_name: l_name,
        }
    }
}
