use actix_web::{
    HttpResponse, get,
    web::{Data, Query},
};
use anyhow::Context;
use serde::Deserialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::profile::ProfileError;

#[derive(Deserialize, Debug)]
pub struct Parameters {
    profile_token: String,
}

#[tracing::instrument(name = "Confirm a pending profile" skip(parameters, pool))]
#[utoipa::path(get, path = "/profile/confirm", params(("profile_token" = String, Query, description="Profile Token")),)]
#[get("/profile/confirm")]
pub async fn confirm_profile(
    parameters: Query<Parameters>,
    pool: Data<PgPool>,
) -> Result<HttpResponse, ProfileError> {
    let id = get_profile_id_from_token(&pool, &parameters.profile_token)
        .await
        .context("Associated profile id not found")?;

    match id {
        None => Ok(HttpResponse::Unauthorized().finish()),
        Some(profile_id) => {
            confirm_subscriber(&pool, profile_id)
                .await
                .context("failed to confirm new profile")?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(name = "Mark Profile as Confirmed", skip(profile_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, profile_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE profile SET status = $1 WHERE id= $2")
        .bind("confirmed")
        .bind(profile_id)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {e:?}");
            e
        })?;

    Ok(())
}

#[tracing::instrument(name = "Get profile_id from token", skip(pool, profile_token))]
pub async fn get_profile_id_from_token(
    pool: &PgPool,
    profile_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query("SELECT profile_id FROM profile_tokens WHERE profile_token= $1")
        .bind(profile_token)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {e:?}");
            e
        })?;
    Ok(result.map(|r| r.get::<Uuid, _>("profile_id")))
}
