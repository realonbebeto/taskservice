use utoipa::OpenApi;

// API Configuration and Documentation
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "RUST CRUD API",description="RUST Actix-web and SQLX CRUD API")
    ),
    paths(
        crate::api::task::get_task,
        crate::api::task::submit_task,
        crate::api::task::start_task,
        crate::api::task::pause_task,
        crate::api::task::complete_task,
        crate::api::task::fail_task,
        crate::api::profile::get_profile,
        crate::api::profile::create_profile,
        crate::api::profile::update_profile,
        crate::api::profile::delete_profile

    )
)]
pub struct ApiDoc;
