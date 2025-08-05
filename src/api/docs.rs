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
        crate::api::user::get_user,
        crate::api::user::create_user,
        crate::api::user::update_user,
        crate::api::user::delete_user

    )
)]
pub struct ApiDoc;
