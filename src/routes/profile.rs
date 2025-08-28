use crate::email_client::EmailClient;
use crate::error::profile::ProfileError;
use crate::error::store_token::StoreTokenError;
use crate::model::profile::{
    Profile, ProfileCreateRequest, ProfileIdentifier, ProfileResponse, ProfileUpdate,
};
use crate::repository::pgdb;
use crate::startup::ApplicationBaseUri;
use crate::util::token_generator::generate_profile_token;
use actix_web::{
    HttpResponse, delete, get, post, put,
    web::{Data, Json, Path},
};

use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};

#[utoipa::path(get, path = "/profiles/{id:[0-9a-fA-F-]{36}}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, body=ProfileResponse, description="Profile found"), (status=404, description="No Profile Found"),))]
#[get("/profiles/{id:[0-9a-fA-F-]{36}}")]
pub async fn get_profile(
    pool: Data<PgPool>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<Json<ProfileResponse>, ProfileError> {
    let prf = pgdb::db_get_profile(pool.get_ref(), &profile_identifier.into_inner().id)
        .await
        .context("Associated profile not found")?;

    Ok(Json(prf))
}

#[tracing::instrument(
    name = "Sending a confirmation email to a new profile",
    skip(email_client, profile)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    profile: Profile,
    base_uri: &str,
    profile_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/profile/confirm?profile_token={}",
        base_uri, profile_token
    );

    email_client
        .send_email(
            &profile.email,
            "Welcome",
            &format!(
                "Welcome to our newsletter!<br/>\
            Click <a href=\"{}\">here</a> to confirm your account.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!\nVisit {} to confirm your account",
                confirmation_link,
            ),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to send confirmation email {e:?}",);
            e
        })?;

    Ok(())
}

#[tracing::instrument(name = "Store profile token in the database", skip(profile_token, tx))]
pub async fn store_token(
    tx: &mut Transaction<'_, Postgres>,
    profile: &Profile,
    profile_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query("INSERT INTO profile_tokens(profile_token, profile_id) VALUES($1, $2)")
        .bind(profile_token)
        .bind(profile.id)
        .execute(&mut **tx)
        .await
        .map_err(StoreTokenError)?;

    Ok(())
}

#[tracing::instrument(name = "Registering a new profile", 
skip(pool, request, email_client),
fields(profile_fname=%request.first_name, profile_email=%request.email, profile_username=%request.username)
)]
#[utoipa::path(post, path = "/profile",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[post("/profile")]
pub async fn create_profile(
    pool: Data<PgPool>,
    request: Json<ProfileCreateRequest>,
    email_client: Data<EmailClient>,
    base_uri: Data<ApplicationBaseUri>,
) -> Result<HttpResponse, ProfileError> {
    let profile: Profile = request
        .into_inner()
        .try_into()
        .map_err(|e: anyhow::Error| ProfileError::ValidationError(e.to_string()))?;

    // Check if the profile already exists
    let r = pgdb::db_get_profile(&pool, &profile.id).await;

    if r.is_ok() {
        return Ok(HttpResponse::Conflict().finish());
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    pgdb::db_create_profile(&mut transaction, &profile)
        .await
        .context("Failed to insert new profile in the database")?;

    let profile_token = generate_profile_token();

    store_token(&mut transaction, &profile, &profile_token)
        .await
        .context("Failed to store the confirmation token for a new profile.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store new profile")?;

    send_confirmation_email(&email_client, profile, &base_uri.0, &profile_token)
        .await
        .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().body(profile_token))
}

#[utoipa::path(post, path = "/profile/update",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[put("/profile/update")]
pub async fn update_profile(
    pool: Data<PgPool>,
    request: Json<ProfileUpdate>,
) -> Result<Json<ProfileUpdate>, ProfileError> {
    let p_update = ProfileUpdate::new(
        &request.id,
        request.first_name.as_deref(),
        request.last_name.as_deref(),
    );
    pgdb::db_update_profile(pool.get_ref(), &p_update)
        .await
        .context("Failed to update profile")?;

    Ok(Json(p_update))
}

#[utoipa::path(delete, path = "/profile/{id}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, description="User deletion successful"), (status=404, description="User deletion unsuccessful"),))]
#[delete("/profile/{id}")]
pub async fn delete_profile(
    pool: Data<PgPool>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<HttpResponse, ProfileError> {
    pgdb::delete_profile(pool.get_ref(), &profile_identifier.into_inner().id)
        .await
        .context("Failed to delete profile")?;

    Ok(HttpResponse::Ok().body("Deletion successful"))
}
