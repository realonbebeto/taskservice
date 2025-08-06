use crate::model::profile::{Profile, ProfileUpdate};
use crate::repository::pgdb::PGDBRepository;
use actix_web::{
    HttpResponse, ResponseError, delete, get,
    http::{StatusCode, header::ContentType},
    post, put,
    web::{Data, Json, Path},
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ProfileIdentifier {
    id: String,
}

#[derive(Debug, Display)]
pub enum ProfileError {
    ProfileNotFound,
    ProfileUpdateFailure,
    ProfileCreationFailure,
    ProfileDeletionFailure,
}

impl ResponseError for ProfileError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match self {
            ProfileError::ProfileNotFound => StatusCode::NOT_FOUND,
            &ProfileError::ProfileUpdateFailure => StatusCode::FAILED_DEPENDENCY,
            ProfileError::ProfileCreationFailure => StatusCode::BAD_REQUEST,
            ProfileError::ProfileDeletionFailure => StatusCode::FAILED_DEPENDENCY,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct ProfileCreateRequest {
    first_name: String,
    last_name: String,
}

#[utoipa::path(get, path = "/profile/{id}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, body=Profile, description="Profile found"), (status=404, description="No Profile Found"),))]
#[get("/profile/{id}")]
pub async fn get_profile(
    ddb_repo: Data<PGDBRepository>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<Json<Profile>, ProfileError> {
    let prf = ddb_repo
        .get_profile(&profile_identifier.into_inner().id)
        .await;

    match prf {
        Some(prf) => Ok(Json(prf)),
        None => Err(ProfileError::ProfileNotFound),
    }
}

#[utoipa::path(post, path = "/profile",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[post("/profile")]
pub async fn create_profile(
    ddb_repo: Data<PGDBRepository>,
    request: Json<ProfileCreateRequest>,
) -> Result<Json<Profile>, ProfileError> {
    let profile = Profile::new(&request.first_name, &request.last_name);

    let result = ddb_repo.create_profile(&profile).await;

    match result {
        Ok(_) => Ok(Json(profile)),
        Err(e) => {
            eprintln!("{e:?}");
            Err(ProfileError::ProfileCreationFailure)
        }
    }
}

#[utoipa::path(post, path = "/profile/update",
responses((status=200, body=Profile, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[put("/profile/update")]
pub async fn update_profile(
    ddb_repo: Data<PGDBRepository>,
    request: Json<ProfileUpdate>,
) -> Result<Json<ProfileUpdate>, ProfileError> {
    let p_update = ProfileUpdate::new(
        &request.id,
        request.first_name.as_deref(),
        request.last_name.as_deref(),
    );
    let result = ddb_repo.update_profile(&p_update).await;

    match result {
        Ok(_) => Ok(Json(p_update)),
        Err(e) => {
            eprintln!("{e:?}");
            Err(ProfileError::ProfileUpdateFailure)
        }
    }
}

#[utoipa::path(delete, path = "/profile/{id}",
params(("id" = String, Path, description="Profile Id")),
responses((status=200, description="User deletion successful"), (status=404, description="User deletion unsuccessful"),))]
#[delete("/profile/{id}")]
pub async fn delete_profile(
    ddb_repo: Data<PGDBRepository>,
    profile_identifier: Path<ProfileIdentifier>,
) -> Result<Json<String>, ProfileError> {
    let result = ddb_repo
        .delete_profile(&profile_identifier.into_inner().id)
        .await;

    match result {
        Ok(_) => Ok(Json("Profile deletion successful".into())),
        Err(e) => {
            eprintln!("{e:?}");
            Err(ProfileError::ProfileDeletionFailure)
        }
    }
}
