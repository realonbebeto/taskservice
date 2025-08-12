use sqlx::PgPool;
use std::net::TcpListener;
use taskservice::configuration::get_configuration;
use taskservice::startup::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration");
    let pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    // Port is coming from the settings
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, pool)?.await
}
