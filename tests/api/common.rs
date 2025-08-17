use std::collections::HashMap;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use taskservice::configuration::{DatabaseSettings, get_configuration};
use taskservice::startup::{Application, get_connection_pool};
use taskservice::telemetry::{get_tracing_subscriber, init_tracing_subscriber};
use uuid::Uuid;
use wiremock::MockServer;

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

#[allow(unused)]
pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
    pub connection: PgConnection,
    pub db_name: String,
    pub email_server: MockServer,
    pub port: u16,
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

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["Html-part"].as_str().unwrap());
        let plain_text = get_link(&body["Text-part"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    pub async fn drop_test_db(&mut self) {
        self.connection
            .execute(format!(r#"DROP DATABASE "{}" WITH (FORCE);"#, self.db_name).as_str())
            .await
            .expect("Failed to create database.");
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

    // Launch a mock server to stand in for Email(s) API
    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        // Use a different database for each test case
        c.database.db_name = Uuid::new_v4().to_string();

        c.application.port = 0;

        // Use the mock server as email API
        c.email_client.base_uri = email_server.uri();

        c
    };

    // Create and migrate the database
    let (_, connection) = configure_database(&configuration.database).await;

    let application = Application::build(&configuration)
        .await
        .expect("Failed to build application");

    let port = application.port();
    let address = format!("http://127.0.0.1:{}", port);
    let _ = tokio::spawn(application.run_until_stopped());

    dbg!(&address);

    TestApp {
        address,
        pool: get_connection_pool(&configuration.database),
        connection,
        db_name: configuration.database.db_name,
        email_server,
        port,
    }
}

async fn configure_database(config: &DatabaseSettings) -> (PgPool, PgConnection) {
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

    (connection_pool, connection)
}
