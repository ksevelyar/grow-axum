use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::AppError;
use crate::grow_logs::{
    GrowLog, GrowLogDetail, GrowLogSummary, list_log_photo_storage_keys, list_user_logs,
    parse_timestamp, remove_photo_files, validate_date_order, validate_nonempty_field,
};
use crate::handlers::request::decode_json_request;

#[derive(Debug, Deserialize)]
pub struct GrowLogPayload {
    pub plant_id: i64,
    pub name: String,
    pub started_at: String,
    pub description: Option<String>,
    pub ended_at: Option<String>,
    pub outcome: Option<String>,
}

pub async fn index(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<GrowLogSummary>>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    Ok(Json(
        list_user_logs(&state.pool, authenticated_user.id).await?,
    ))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    payload: Result<Json<GrowLogPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<GrowLogDetail>), AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload = decode_json_request(payload)?;
    validate_nonempty_field("name", &payload.name)?;
    let started_at = parse_timestamp("started_at", &payload.started_at)?;
    let ended_at = parse_optional_timestamp("ended_at", payload.ended_at.as_deref())?;
    validate_date_order(started_at, ended_at)?;
    crate::grow_logs::find_plant(&state.pool, authenticated_user.id, payload.plant_id).await?;
    let grow_log = insert_grow_log(
        &state.pool,
        authenticated_user.id,
        payload,
        started_at,
        ended_at,
    )
    .await?;
    let grow_log_detail =
        crate::grow_logs::find_log_detail(&state.pool, authenticated_user.id, grow_log.id).await?;
    Ok((StatusCode::CREATED, Json(grow_log_detail)))
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(log_id): Path<i64>,
) -> Result<Json<GrowLogDetail>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let detail =
        crate::grow_logs::find_log_detail(&state.pool, authenticated_user.id, log_id).await?;
    Ok(Json(detail))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(log_id): Path<i64>,
    payload: Result<Json<GrowLogPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<GrowLogDetail>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload = decode_json_request(payload)?;
    crate::grow_logs::find_log(&state.pool, authenticated_user.id, log_id).await?;
    validate_nonempty_field("name", &payload.name)?;
    let started_at = parse_timestamp("started_at", &payload.started_at)?;
    let ended_at = parse_optional_timestamp("ended_at", payload.ended_at.as_deref())?;
    validate_date_order(started_at, ended_at)?;
    crate::grow_logs::find_plant(&state.pool, authenticated_user.id, payload.plant_id).await?;
    sqlx::query(
        "UPDATE grow_logs
         SET plant_id = $1, name = $2, started_at = $3, description = $4,
             ended_at = $5, outcome = $6, updated_at = NOW()
         WHERE id = $7 AND user_id = $8",
    )
    .bind(payload.plant_id)
    .bind(payload.name.trim())
    .bind(started_at)
    .bind(payload.description)
    .bind(ended_at)
    .bind(payload.outcome)
    .bind(log_id)
    .bind(authenticated_user.id)
    .execute(&state.pool)
    .await?;
    let detail =
        crate::grow_logs::find_log_detail(&state.pool, authenticated_user.id, log_id).await?;
    Ok(Json(detail))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(log_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::grow_logs::find_log(&state.pool, authenticated_user.id, log_id).await?;
    let mut transaction = state.pool.begin().await?;
    let storage_keys =
        list_log_photo_storage_keys(&mut transaction, authenticated_user.id, log_id).await?;
    sqlx::query("DELETE FROM grow_logs WHERE id = $1 AND user_id = $2")
        .bind(log_id)
        .bind(authenticated_user.id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    remove_photo_files(&state.photo_storage_directory, storage_keys).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn insert_grow_log(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: GrowLogPayload,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<GrowLog, AppError> {
    Ok(sqlx::query_as::<_, GrowLog>(
        "INSERT INTO grow_logs (user_id, plant_id, name, started_at, description, ended_at, outcome)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, plant_id, name, started_at, description, ended_at, outcome,
                   inserted_at, updated_at",
    )
    .bind(user_id)
    .bind(payload.plant_id)
    .bind(payload.name.trim())
    .bind(started_at)
    .bind(payload.description)
    .bind(ended_at)
    .bind(payload.outcome)
    .fetch_one(pool)
    .await?)
}

fn parse_optional_timestamp(
    field_name: &str,
    timestamp_value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, AppError> {
    timestamp_value
        .map(|timestamp_text| parse_timestamp(field_name, timestamp_text))
        .transpose()
}
