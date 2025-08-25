use crate::domain::{
    email::ProfileEmail, name::ProfileName, password::Password, username::ProfileUsername,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow, Debug, ToSchema)]
pub struct Profile {
    pub id: Uuid,
    pub first_name: ProfileName,
    pub last_name: ProfileName,
    pub email: ProfileEmail,
    pub username: ProfileUsername,
    pub password: Password,
}

impl TryFrom<ProfileCreateRequest> for Profile {
    type Error = String;

    fn try_from(value: ProfileCreateRequest) -> Result<Self, Self::Error> {
        let first_name = ProfileName::parse(value.first_name)?;
        let last_name = ProfileName::parse(value.last_name)?;
        let email = ProfileEmail::parse(value.email)?;
        let username = ProfileUsername::parse(value.username)?;
        let password = Password::parse(value.password)?;

        Ok(Profile {
            id: Uuid::new_v4(),
            first_name,
            last_name,
            email,
            username,
            password,
        })
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

#[derive(Serialize, Deserialize, FromRow, Debug, ToSchema)]
pub struct ProfileResponse {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ProfileIdentifier {
    pub id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct ProfileCreateRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub username: String,
    pub password: String,
}
