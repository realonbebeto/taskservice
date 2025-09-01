use std::fmt::{Debug, Display};

use taskservice::configuration::get_configuration;
use taskservice::issue_delivery::run_delivery_worker_until_stopped;
use taskservice::startup::Application;
use taskservice::telemetry::{get_tracing_subscriber, init_tracing_subscriber};
use tokio::task::JoinError;

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{} task failed to complete", task_name)
        }
        Err(e) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{} failed", task_name)
        }
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_tracing_subscriber("taskservice".into(), "info".into(), std::io::stdout);
    init_tracing_subscriber(subscriber);
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration");
    let application = Application::build(&configuration).await?;
    let application_task = tokio::spawn(application.run_until_stopped());
    let delivery_worker = tokio::spawn(run_delivery_worker_until_stopped(configuration));

    tokio::select! {o = application_task => {report_exit("API", o);}, o = delivery_worker => {report_exit("delivery_worker", o);}};
    Ok(())
}
