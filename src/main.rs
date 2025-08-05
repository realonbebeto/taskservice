mod api;
mod model;
mod repository;

use actix_web::web;
use actix_web::{App, HttpServer, middleware::Logger, web::Data};
use api::task::{complete_task, fail_task, get_task, pause_task, start_task, submit_task};
use api::user::{create_user, delete_user, get_user, update_user};
use repository::pgdb::PGDBRepository;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    env_logger::init();

    HttpServer::new(move || {
        let pgdb_repo = PGDBRepository::init("task".to_string());
        let ddb_data = Data::new(pgdb_repo);

        let logger = Logger::default();
        let openapi = api::docs::ApiDoc::openapi();
        App::new()
            .wrap(logger)
            .app_data(ddb_data)
            .route("/", web::get().to(api::index::index_page))
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", openapi.clone()))
            .service(get_task)
            .service(pause_task)
            .service(complete_task)
            .service(start_task)
            .service(submit_task)
            .service(fail_task)
            .service(create_user)
            .service(delete_user)
            .service(get_user)
            .service(update_user)
    })
    .bind(("127.0.0.1", 80))?
    .run()
    .await
}
