use std::collections::HashMap;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use taskservice::configuration::{DatabaseSettings, get_configuration};
use taskservice::startup::{Application, get_connection_pool};
use taskservice::telemetry::{get_tracing_subscriber, init_tracing_subscriber};
use uuid::Uuid;

#[allow(unused)]
pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
}

impl TestApp {
    pub async fn post_profiles(
        &self,
        body: &HashMap<&'static str, &'static str>,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/profile", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriper_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            get_tracing_subscriber(subscriper_name, default_filter_level, std::io::stdout);
        init_tracing_subscriber(subscriber);
    } else {
        let subscriber =
            get_tracing_subscriber(subscriper_name, default_filter_level, std::io::sink);
        init_tracing_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    // Telemetry by tracing and Lazy
    Lazy::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        // Use a different database for each test case
        c.database.db_name = Uuid::new_v4().to_string();

        c.application.port = 0;

        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let application = Application::build(&configuration)
        .await
        .expect("Failed to build application");

    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        pool: get_connection_pool(&configuration.database),
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
