use utoipa::OpenApi;

// API Configuration and Documentation
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "RUST CRUD API",description="RUST Actix-web and SQLX CRUD API")
    ),
    paths(
        crate::routes::health_check::health_check,
        crate::routes::task::get_task,
        crate::routes::task::submit_task,
        crate::routes::task::start_task,
        crate::routes::task::pause_task,
        crate::routes::task::complete_task,
        crate::routes::task::fail_task,
        crate::routes::profile::get_profile,
        crate::routes::profile::create_profile,
        crate::routes::profile::update_profile,
        crate::routes::profile::delete_profile,
        crate::routes::profile_confirm::confirm_profile

    )
)]
pub struct ApiDoc;
