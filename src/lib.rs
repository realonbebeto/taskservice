mod api;
mod model;
mod repository;

use actix_web::web;
use actix_web::{App, HttpServer, middleware::Logger, web::Data};
use api::profile::{create_profile, delete_profile, get_profile, update_profile};
use api::task::{complete_task, fail_task, get_task, pause_task, start_task, submit_task};
use repository::pgdb::PGDBRepository;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub async fn run() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    env_logger::init();

    HttpServer::new(move || {
        let pgdb_repo = PGDBRepository::init();
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
            .service(create_profile)
            .service(delete_profile)
            .service(get_profile)
            .service(update_profile)
    })
    .bind(("127.0.0.1", 80))?
    .run()
    .await
}
