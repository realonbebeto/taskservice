use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes;
use crate::routes::health_check::health_check;
use crate::routes::profile::{create_profile, delete_profile, get_profile, update_profile};
use crate::routes::task::{
    complete_task, fail_task, get_task, pause_task, start_task, submit_task,
};
use actix_web::dev::Server;
use actix_web::web;
use actix_web::{App, HttpServer, web::Data};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn run(
    listener: TcpListener,
    pg_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    unsafe {
        // std::env::set_var("RUST_LOG", "trace");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let pg_pool = Data::new(pg_pool);
    let email_client = Data::new(email_client);

    let server = HttpServer::new(move || {
        // let pgdb_repo = PGDBRepository::init();

        let logger = TracingLogger::default();
        let openapi = routes::docs::ApiDoc::openapi();
        App::new()
            .wrap(logger)
            .app_data(pg_pool.clone())
            .app_data(email_client.clone())
            .route("/", web::get().to(routes::index::index_page))
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
            .service(health_check)
            .service(get_task)
            .service(pause_task)
            .service(complete_task)
            .service(start_task)
            .service(submit_task)
            .service(fail_task)
            .service(create_profile)
            .service(delete_profile)
            .service(get_profile)
            .service(update_profile)
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
    pub async fn build(configuration: &Settings) -> Result<Self, std::io::Error> {
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
        let server = run(listener, pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
