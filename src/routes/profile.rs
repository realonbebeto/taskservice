use crate::model::profile::{
    Profile, ProfileCreateRequest, ProfileIdentifier, ProfileResponse, ProfileUpdate,
};
use crate::repository::pgdb;
use actix_web::{
    HttpResponse, ResponseError, delete, get,
    http::{StatusCode, header::ContentType},
    post, put,
    web::{Data, Json, Path},
};
use derive_more::Display;
use sqlx::PgPool;

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

#[utoipa::path(get, path = "/profile/{id}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, body=ProfileResponse, description="Profile found"), (status=404, description="No Profile Found"),))]
#[get("/profile/{id}")]
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

#[tracing::instrument(name = "Registering a new profile", 
skip(pool, request),
fields(profile_fname=%request.first_name, profile_email=%request.email)
)]
#[utoipa::path(post, path = "/profile",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[post("/profile")]
pub async fn create_profile(
    pool: Data<PgPool>,
    request: Json<ProfileCreateRequest>,
) -> HttpResponse {
    let profile = match request.into_inner().try_into() {
        Ok(profile) => profile,
        Err(msg) => return HttpResponse::BadRequest().body(msg),
    };

    tracing::info!("Creating new profile in the database",);
    let result = pgdb::db_create_profile(pool.get_ref(), &profile).await;

    match result {
        Ok(_) => {
            tracing::info!("New profile has been saved");
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("Failed to save new profile {e:?}",);
            HttpResponse::InternalServerError().finish()
        }
    }
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
