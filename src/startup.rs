use crate::routes;
use crate::routes::health_check::health_check;
use crate::routes::profile::{create_profile, delete_profile, get_profile, update_profile};
use crate::routes::task::{
    complete_task, fail_task, get_task, pause_task, start_task, submit_task,
};
use actix_web::dev::Server;
use actix_web::web;
use actix_web::{App, HttpServer, middleware::Logger, web::Data};
use sqlx::PgPool;
use std::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn run(listener: TcpListener, pg_pool: PgPool) -> std::io::Result<Server> {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let pg_pool = Data::new(pg_pool);

    if let Err(e) = env_logger::try_init() {
        eprintln!("Logger already initialized: {}", e);
    };

    let server = HttpServer::new(move || {
        // let pgdb_repo = PGDBRepository::init();

        let logger = Logger::default();
        let openapi = routes::docs::ApiDoc::openapi();
        App::new()
            .wrap(logger)
            .app_data(pg_pool.clone())
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
