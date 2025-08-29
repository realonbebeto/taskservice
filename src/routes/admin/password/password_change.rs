use actix_web::{HttpResponse, web};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::authentication::{Credentials, update_password, validate_credentials};
use crate::domain::id::ProfileId;
use crate::domain::password::Password;
use crate::error::authentication::AuthError;
use crate::error::authentication::StdResponse;
use crate::repository::pgdb::get_username;
use crate::session_state::TypedSession;
use crate::util::e500;

use secrecy::SecretBox;

#[derive(Deserialize, ToSchema)]
pub struct PasswordChange {
    current_password: String,
    new_password: String,
    new_password_check: String,
}

#[tracing::instrument(name = "Change Password", skip(form, pool))]
#[utoipa::path(post, path = "/admin/password", responses((status=200, description="Change successful"), (status=303, description="Wrong Entry"), (status=500, description="Something went wrong on our end")))]
pub async fn change_password(
    form: web::Form<PasswordChange>,
    pool: web::Data<PgPool>,
    profile_id: web::ReqData<ProfileId>,
) -> Result<HttpResponse, actix_web::Error> {
    let profile_id = profile_id.0;

    let username = get_username(profile_id, &pool).await.map_err(e500)?;

    if form.0.new_password != form.0.new_password_check {
        return Ok(HttpResponse::UnprocessableEntity().json(StdResponse {
            message: "New Passwords don't Match",
        }));
    }

    if form.0.new_password == form.0.current_password {
        return Ok(HttpResponse::UnprocessableEntity().json(StdResponse {
            message: "New password must be different from the current password",
        }));
    }

    let credentials = Credentials {
        username,
        password: SecretBox::new(Box::new(form.0.current_password)),
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                Ok(HttpResponse::Unauthorized().json(StdResponse {
                    message: "Current Password is Incorrect!",
                }))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    if let Err(e) = Password::parse(form.0.new_password.clone()) {
        return Ok(HttpResponse::UnprocessableEntity().json(StdResponse {
            message: &e.to_string(),
        }));
    }

    update_password(profile_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(StdResponse {
        message: "Password Change Successful",
    }))
}

#[tracing::instrument(name = "Logout", skip(session))]
#[utoipa::path(post, path = "/admin/logout", responses((status=200, description="Logout successful"), (status=303, description="No active session"), (status=500, description="Something went wrong on our end")))]
pub async fn logout(
    profile_id: web::ReqData<ProfileId>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    dbg!(1);
    session.log_out();
    Ok(HttpResponse::Ok().json(StdResponse {
        message: "You have successfully logged out.",
    }))
}
