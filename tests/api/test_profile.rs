use argon2::password_hash::{SaltString, rand_core};
use argon2::{Argon2, Params, PasswordHasher};
use fake::Fake;
use fake::faker::internet::en::{Password, SafeEmail, Username};
use fake::faker::name::en::{FirstName, LastName};
use sqlx::PgPool;
use taskservice::domain::{
    email::ProfileEmail, name::ProfileName, password, username::ProfileUsername,
};
use uuid::Uuid;

pub struct TestProfile {
    pub id: Uuid,
    pub first_name: ProfileName,
    pub last_name: ProfileName,
    pub email: ProfileEmail,
    pub status: String,
    pub username: ProfileUsername,
    pub password: password::Password,
}

impl TestProfile {
    pub fn generate(confirmed: bool) -> Self {
        let status = if confirmed {
            "confirmed".to_string()
        } else {
            "pending_confirmation".to_string()
        };
        Self {
            id: Uuid::new_v4(),
            first_name: ProfileName::parse(FirstName().fake()).unwrap(),
            last_name: ProfileName::parse(LastName().fake()).unwrap(),
            email: ProfileEmail::parse(SafeEmail().fake()).unwrap(),
            status,
            username: ProfileUsername::parse(Username().fake()).unwrap(),
            password: password::Password::parse(
                Password(std::ops::Range { start: 8, end: 16 }).fake(),
            )
            .unwrap(),
        }
    }

    pub async fn store_test_profile(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand_core::OsRng);
        let password = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_ref().as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query(
        "INSERT INTO profile (id, first_name, last_name, email, status, username, password) VALUES($1, $2, $3, $4, $5, $6, $7)",
    ).bind(self.id)
    .bind(self.first_name.as_ref())
    .bind(self.last_name.as_ref())
    .bind(self.email.as_ref())
    .bind(&self.status)
    .bind(self.username.as_ref())
    .bind(password)
    .execute(pool)
    .await
    .expect("Failed to create test user. ");
    }
}
