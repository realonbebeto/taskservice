use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes;
use crate::routes::admin::dashboard::admin_dashboard;
use crate::routes::health_check::health_check;
use crate::routes::login::{log_in, log_in_check};
use crate::routes::profile::{create_profile, delete_profile, get_profile, update_profile};
use crate::routes::profile_confirm::confirm_profile;
use crate::routes::task::{
    complete_task, create_task, fail_task, get_task, pause_task, start_task,
};
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::web;
use actix_web::{App, HttpServer, web::Data};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug)]
pub struct ApplicationBaseUri(pub String);

async fn run(
    listener: TcpListener,
    pg_pool: PgPool,
    email_client: EmailClient,
    base_uri: &str,
    secret: &str,
    redis_uri: &str,
) -> Result<Server, anyhow::Error> {
    unsafe {
        // std::env::set_var("RUST_LOG", "trace");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let pg_pool = Data::new(pg_pool);
    let email_client = Data::new(email_client);
    let base_uri = Data::new(ApplicationBaseUri(base_uri.to_string()));
    let secret_key = Key::from(secret.as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri).await?;

    let server = HttpServer::new(move || {
        // let pgdb_repo = PGDBRepository::init();

        let logger = TracingLogger::default();
        let openapi = routes::docs::ApiDoc::openapi();
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(logger)
            .app_data(pg_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_uri.clone())
            .route("/", web::get().to(routes::index::index_page))
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
            .service(health_check)
            .service(get_task)
            .service(pause_task)
            .service(complete_task)
            .service(start_task)
            .service(create_task)
            .service(fail_task)
            .service(create_profile)
            .service(delete_profile)
            .service(confirm_profile)
            .service(get_profile)
            .service(update_profile)
            .service(log_in)
            .service(log_in_check)
            .service(admin_dashboard)
    })
    .listen(listener)?
    .run();

    Ok(server)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: &Settings) -> Result<Self, anyhow::Error> {
        let pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            &configuration.email_client.base_uri,
            sender_email,
            &configuration.email_client.private_email_key,
            &configuration.email_client.public_email_key,
            timeout,
        );

        // Port is coming from the settings
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            pool,
            email_client,
            &configuration.application.app_uri,
            &configuration.application.hmac_secret,
            &configuration.redis_uri,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
