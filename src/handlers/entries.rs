use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};
use crate::grow_logs::{
    Entry, EntryWithPhotos, list_entry_photo_storage_keys, parse_timestamp, remove_photo_files,
};
use crate::handlers::request::decode_json_request;

const VALID_STAGES: [&str; 4] = ["germination", "seedling", "vegetation", "bloom"];

#[derive(Debug, Deserialize)]
pub struct EntryPayload {
    pub occurred_at: String,
    pub stage: String,
    pub description: Option<String>,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(log_id): Path<i64>,
    payload: Result<Json<EntryPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<EntryWithPhotos>), AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::grow_logs::find_log(&state.pool, authenticated_user.id, log_id).await?;
    let payload = decode_json_request(payload)?;
    let occurred_at = parse_timestamp("occurred_at", &payload.occurred_at)?;
    validate_stage(&payload.stage)?;
    let entry = insert_entry(
        &state.pool,
        log_id,
        occurred_at,
        &payload.stage,
        payload.description,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(EntryWithPhotos {
            entry,
            photos: Vec::new(),
        }),
    ))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(entry_id): Path<i64>,
    payload: Result<Json<EntryPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<EntryWithPhotos>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::grow_logs::find_entry(&state.pool, authenticated_user.id, entry_id).await?;
    let payload = decode_json_request(payload)?;
    let occurred_at = parse_timestamp("occurred_at", &payload.occurred_at)?;
    validate_stage(&payload.stage)?;
    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE grow_log_entries
         SET occurred_at = $1, stage = $2, description = $3, updated_at = NOW()
         WHERE id = $4
         RETURNING id, grow_log_id, occurred_at, stage, description, inserted_at, updated_at",
    )
    .bind(occurred_at)
    .bind(payload.stage)
    .bind(payload.description)
    .bind(entry_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let photos = sqlx::query_as::<_, PhotoRow>(
        "SELECT id, media_type, original_filename, inserted_at, updated_at
         FROM grow_log_entry_photos WHERE grow_log_entry_id = $1 ORDER BY id",
    )
    .bind(entry_id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(PhotoRow::into_photo)
    .collect();
    Ok(Json(EntryWithPhotos { entry, photos }))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(entry_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::grow_logs::find_entry(&state.pool, authenticated_user.id, entry_id).await?;
    let mut transaction = state.pool.begin().await?;
    let storage_keys =
        list_entry_photo_storage_keys(&mut transaction, authenticated_user.id, entry_id).await?;
    sqlx::query("DELETE FROM grow_log_entries WHERE id = $1")
        .bind(entry_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    remove_photo_files(&state.photo_storage_directory, storage_keys).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn insert_entry(
    pool: &sqlx::PgPool,
    log_id: i64,
    occurred_at: DateTime<Utc>,
    stage: &str,
    description: Option<String>,
) -> Result<Entry, AppError> {
    Ok(sqlx::query_as::<_, Entry>(
        "INSERT INTO grow_log_entries (grow_log_id, occurred_at, stage, description)
         VALUES ($1, $2, $3, $4)
         RETURNING id, grow_log_id, occurred_at, stage, description, inserted_at, updated_at",
    )
    .bind(log_id)
    .bind(occurred_at)
    .bind(stage)
    .bind(description)
    .fetch_one(pool)
    .await?)
}

fn validate_stage(stage: &str) -> Result<(), AppError> {
    if VALID_STAGES.contains(&stage) {
        return Ok(());
    }
    Err(AppError::Validation(vec![FieldError {
        field: "stage".to_owned(),
        message: "must be germination, seedling, vegetation, or bloom".to_owned(),
    }]))
}

#[derive(Debug, sqlx::FromRow)]
struct PhotoRow {
    id: i64,
    media_type: String,
    original_filename: Option<String>,
    inserted_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PhotoRow {
    fn into_photo(self) -> crate::grow_logs::Photo {
        crate::grow_logs::Photo {
            id: self.id,
            media_type: self.media_type,
            original_filename: self.original_filename,
            url: format!("/photos/{}", self.id),
            thumbnail_url: format!("/photos/{}/thumbnail", self.id),
            inserted_at: self.inserted_at,
            updated_at: self.updated_at,
        }
    }
}
