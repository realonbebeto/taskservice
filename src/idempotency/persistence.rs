use super::IdempotencyKey;
use actix_web::{HttpResponse, body::to_bytes, http::StatusCode};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

#[derive(Debug, sqlx::FromRow)]
struct SavedResponse {
    response_status_code: Option<i16>,
    response_headers: Option<Vec<HeaderPairRecord>>,
    response_body: Option<Vec<u8>>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    profile_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
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

    if let Some(sr) = saved_response {
        let status_code = StatusCode::from_u16(sr.response_status_code.unwrap().try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for HeaderPairRecord { name, value } in sr.response_headers.unwrap() {
            response.append_header((name, value));
        }
        Ok(Some(response.body(sr.response_body.unwrap())))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut tx: Transaction<'static, Postgres>,
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
        "UPDATE idempotency 
                SET response_status_code = $3, 
                    response_headers = $4, 
                    response_body = $5,
                    updated_at = now()
                WHERE profile_id =$1 
                AND idempotency_key = $2",
    )
    .bind(profile_id)
    .bind(idempotency_key.as_ref())
    .bind(status_code)
    .bind(headers)
    .bind(body.as_ref())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let http_res = response_head.set_body(body).map_into_boxed_body();
    Ok(http_res)
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_idem_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    profile_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let n_inserted_rows = sqlx::query(
        "INSERT INTO idempotency (profile_id, idempotency_key) 
    VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(profile_id)
    .bind(idempotency_key.as_ref())
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, profile_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we did not find it."))?;

        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
