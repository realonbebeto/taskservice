use crate::session_state::TypedSession;
use actix_web::{HttpResponse, get, web};
use sqlx::PgPool;

use crate::error::authentication::StdResponse;
use crate::repository::pgdb::get_username;
use crate::util::e500;

#[tracing::instrument(name = "Logging In", skip(session, pool))]
#[utoipa::path(get, path = "/admin/dashboard", responses((status=200, description="Authentication successful"), (status=401, description="Authentication failed")))]
#[get("/admin/dashboard")]
pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(profile_id) = session.get_profile_id().map_err(e500)? {
        get_username(profile_id, &pool).await.map_err(e500)?
    } else {
        String::from("Welcome Person of the Internet")
    };

    Ok(HttpResponse::Ok().json(StdResponse {
        message: &format!("Welcome {}", username),
    }))
}
