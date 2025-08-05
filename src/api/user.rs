use actix_web::{HttpResponse, Responder, delete, get, post, put};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Debug, Serialize, Deserialize)]
pub struct CustomResponse {
    pub body: String,
}

#[utoipa::path(get, path = "/user",
responses((status=200, body=CustomResponse, description="User"), (status=404, description="No User Found"),))]
#[get("/user")]
pub async fn get_user() -> impl Responder {
    HttpResponse::Ok().body("get_user")
}

#[utoipa::path(post, path = "/user",
responses((status=200, body=CustomResponse, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[post("/user")]
pub async fn create_user() -> impl Responder {
    HttpResponse::Ok().body("create_user")
}

#[utoipa::path(post, path = "/user",
responses((status=200, body=CustomResponse, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[put("/user")]
pub async fn update_user() -> impl Responder {
    HttpResponse::Ok().json("update_user")
}

#[utoipa::path(delete, path = "/user",
responses((status=200, body=CustomResponse, description="User creation successful"), (status=404, description="User creation unsuccessful"),))]
#[delete("/user")]
pub async fn delete_user() -> impl Responder {
    HttpResponse::Ok().body("delete_user")
}
