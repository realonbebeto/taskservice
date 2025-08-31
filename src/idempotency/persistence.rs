use super::IdempotencyKey;
use actix_web::{HttpResponse, body::to_bytes, http::StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

#[derive(Debug, sqlx::FromRow)]
struct SavedResponse {
    response_status_code: i16,
    response_headers: Vec<HeaderPairRecord>,
    response_body: Vec<u8>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    profile_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    dbg!(1);
    let saved_response = sqlx::query_as::<_, SavedResponse>(
        "SELECT response_status_code, response_headers, response_body 
                                                        FROM idempotency 
                                                        WHERE profile_id = $1 
                                                        AND idempotency_key= $2",
    )
    .bind(profile_id)
    .bind(idempotency_key.as_ref())
    .fetch_optional(pool)
    .await?;

    dbg!(2);

    if let Some(sr) = saved_response {
        let status_code = StatusCode::from_u16(sr.response_status_code.try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for HeaderPairRecord { name, value } in sr.response_headers {
            response.append_header((name, value));
        }
        Ok(Some(response.body(sr.response_body)))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    profile_id: Uuid,
    http_res: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (response_head, body) = http_res.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{}", e))?;
    let status_code = response_head.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers().len());
        for (name, value) in response_head.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    sqlx::query(
        "INSERT INTO idempotency (profile_id, idempotency_key, 
    response_status_code, response_headers, response_body) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(profile_id)
    .bind(idempotency_key.as_ref())
    .bind(status_code)
    .bind(headers)
    .bind(body.as_ref())
    .execute(pool)
    .await?;

    let http_res = response_head.set_body(body).map_into_boxed_body();
    Ok(http_res)
}
