//! src/configuration.rs
use envconfig::Envconfig;
use serde::Deserialize;
use sqlx::ConnectOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode;

use crate::domain::email::ProfileEmail;

#[derive(Deserialize, Envconfig)]
pub struct DatabaseSettings {
    #[envconfig(from = "DB_USERNAME")]
    pub db_username: String,
    #[envconfig(from = "DB_PASSWORD")]
    pub db_password: String,
    #[envconfig(from = "DB_PORT", default = "5432")]
    pub db_port: u16,
    #[envconfig(from = "DB_HOST")]
    pub db_host: String,
    #[envconfig(from = "DB_NAME")]
    pub db_name: String,
    #[envconfig(from = "REQUIRE_SSL")]
    pub require_ssl: bool,
}

#[derive(Deserialize)]
pub enum Environment {
    Local,
    Production,
}

impl std::str::FromStr for Environment {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not supported environment.\
            Use either `local` or `production`.",
                other
            )),
        }
    }
}

#[derive(Deserialize, Envconfig)]
pub struct ApplicationSettings {
    #[envconfig(from = "APP_PORT")]
    pub port: u16,
    #[envconfig(from = "APP_HOST", default = "0.0.0.0")]
    pub host: String,
    #[envconfig(from = "APP_ENVIRONMENT", default = "local")]
    pub app_environment: Environment,
    #[envconfig(from = "APP_URI")]
    pub app_uri: String,
}

#[derive(Deserialize, Envconfig)]
pub struct EmailClientSettings {
    #[envconfig(from = "EMAIL_BASE_URI")]
    pub base_uri: String,
    #[envconfig(from = "SENDER_EMAIL")]
    pub sender_email: String,
    #[envconfig(from = "PUBLIC_EMAIL_KEY")]
    pub public_email_key: String,
    #[envconfig(from = "PRIVATE_EMAIL_KEY")]
    pub private_email_key: String,
    #[envconfig(from = "TIMEOUT_MS")]
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<ProfileEmail, String> {
        ProfileEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

#[derive(Deserialize, Envconfig)]
pub struct Settings {
    #[envconfig(nested)]
    pub database: DatabaseSettings,
    #[envconfig(nested)]
    pub application: ApplicationSettings,
    #[envconfig(nested)]
    pub email_client: EmailClientSettings,
}

pub fn get_configuration() -> Result<Settings, envconfig::Error> {
    // Initialize our configuration reader
    let settings = Settings::init_from_env()
        .expect("Failed to parse required application environment variables");

    // Try to convert the configuration values it read into the Settings type
    Ok(settings)
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.db_host)
            .username(&self.db_username)
            .password(&self.db_password)
            .port(self.db_port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.db_name)
            .log_statements(tracing_log::log::LevelFilter::Trace)
    }
}
