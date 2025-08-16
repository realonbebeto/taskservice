use taskservice::configuration::get_configuration;
use taskservice::startup::Application;
use taskservice::telemetry::{get_tracing_subscriber, init_tracing_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_tracing_subscriber("taskservice".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(subscriber);
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration");
    let application = Application::build(&configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
