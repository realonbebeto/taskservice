use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use taskservice::configuration::get_configuration;
use taskservice::startup::run;
use taskservice::telemetry::{get_tracing_subscriber, init_tracing_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_tracing_subscriber("taskservice".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(subscriber);
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration");
    let pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

    // Port is coming from the settings
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, pool)?.await
}
