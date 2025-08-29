use crate::domain::id::ProfileId;
// use crate::session_state::TypedSession;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

use crate::error::authentication::StdResponse;
use crate::repository::pgdb::get_username;
use crate::util::e500;

#[tracing::instrument(name = "Admin Dashboard", skip(pool))]
#[utoipa::path(get, path = "/admin/dashboard", responses((status=200, description="Authentication successful"), (status=401, description="Authentication failed")))]
// #[get("/admin/dashboard")]
pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    profile_id: web::ReqData<ProfileId>,
) -> Result<HttpResponse, actix_web::Error> {
    dbg!(1);
    let profile_id = profile_id.0;
    let username = get_username(profile_id, &pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok().json(StdResponse {
        message: &format!("Welcome {}", username),
    }))
}
