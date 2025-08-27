use crate::session_state::TypedSession;
use actix_web::{HttpResponse, get, web};
use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::authentication::LoginResponse;

fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get Username", skip(pool))]
async fn get_username(profile_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query("SELECT username FROM profile WHERE id = $1")
        .bind(profile_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform query to retrieve username")?;

    Ok(row.get("username"))
}

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

    Ok(HttpResponse::Ok().json(LoginResponse {
        message: format!("Welcome {}", username),
    }))
}
