use serde::Deserialize;
use sqlx::FromRow;

#[allow(unused)]
#[derive(Deserialize, FromRow, Debug)]
struct Profile {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
}
