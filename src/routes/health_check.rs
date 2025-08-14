use actix_web::HttpResponse;
use actix_web::get;

#[utoipa::path(get,
    path="/health_check",
    responses((status=200, description="Health status")))]
#[get("/health_check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
