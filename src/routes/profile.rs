use crate::email_client::EmailClient;
use crate::model::profile::{
    Profile, ProfileCreateRequest, ProfileIdentifier, ProfileResponse, ProfileUpdate,
};
use crate::repository::pgdb;
use crate::startup::ApplicationBaseUri;
use crate::util::token_generator::generate_profile_token;
use actix_web::{
    HttpResponse, ResponseError, delete, get,
    http::{StatusCode, header::ContentType},
    post, put,
    web::{Data, Json, Path},
};
use derive_more::Display;
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Debug, Display)]
pub enum ProfileError {
    NotFound,
    UpdateFailure,
    CreationFailure,
    DeletionFailure,
}

impl ResponseError for ProfileError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match self {
            ProfileError::NotFound => StatusCode::NOT_FOUND,
            &ProfileError::UpdateFailure => StatusCode::FAILED_DEPENDENCY,
            ProfileError::CreationFailure => StatusCode::BAD_REQUEST,
            ProfileError::DeletionFailure => StatusCode::FAILED_DEPENDENCY,
        }
    }
}

#[utoipa::path(get, path = "/profiles/{id:[0-9a-fA-F-]{36}}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, body=ProfileResponse, description="Profile found"), (status=404, description="No Profile Found"),))]
#[get("/profiles/{id:[0-9a-fA-F-]{36}}")]
pub async fn get_profile(
    pool: Data<PgPool>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<Json<ProfileResponse>, ProfileError> {
    let prf = pgdb::db_get_profile(pool.get_ref(), &profile_identifier.into_inner().id).await;

    match prf {
        Some(prf) => Ok(Json(prf)),
        None => Err(ProfileError::NotFound),
    }
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
            profile.email,
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
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO profile_tokens(profile_token, profile_id) VALUES($1, $2)")
        .bind(profile_token)
        .bind(profile.id)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {e:?}");
            e
        })?;

    Ok(())
}

#[tracing::instrument(name = "Registering a new profile", 
skip(pool, request, email_client),
fields(profile_fname=%request.first_name, profile_email=%request.email)
)]
#[utoipa::path(post, path = "/profile",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[post("/profile")]
pub async fn create_profile(
    pool: Data<PgPool>,
    request: Json<ProfileCreateRequest>,
    email_client: Data<EmailClient>,
    base_uri: Data<ApplicationBaseUri>,
) -> HttpResponse {
    let profile: Profile = match request.into_inner().try_into() {
        Ok(profile) => profile,
        Err(msg) => return HttpResponse::BadRequest().body(msg),
    };
    // Check if the profile already exists
    if pgdb::db_get_profile(&pool, &profile.id).await.is_some() {
        return HttpResponse::Conflict().finish();
    };

    let mut transaction = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    tracing::info!("Creating new profile in the database",);
    if let Err(e) = pgdb::db_create_profile(&mut transaction, &profile).await {
        tracing::error!("Failed to save new profile {e:?}",);
        return HttpResponse::InternalServerError().finish();
    };

    tracing::info!("New profile has been saved");

    let profile_token = generate_profile_token();

    if store_token(&mut transaction, &profile, &profile_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(&email_client, profile, &base_uri.0, &profile_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().body(profile_token)
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
    let result = pgdb::db_update_profile(pool.get_ref(), &p_update).await;

    match result {
        Ok(_) => Ok(Json(p_update)),
        Err(e) => {
            eprintln!("{e:?}");
            Err(ProfileError::UpdateFailure)
        }
    }
}

#[utoipa::path(delete, path = "/profile/{id}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, description="User deletion successful"), (status=404, description="User deletion unsuccessful"),))]
#[delete("/profile/{id}")]
pub async fn delete_profile(
    pool: Data<PgPool>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<Json<String>, ProfileError> {
    let result = pgdb::delete_profile(pool.get_ref(), &profile_identifier.into_inner().id).await;

    match result {
        Ok(_) => Ok(Json("Profile deletion successful".into())),
        Err(e) => {
            eprintln!("{e:?}");
            Err(ProfileError::DeletionFailure)
        }
    }
}
