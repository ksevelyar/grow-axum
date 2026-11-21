use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
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
    let mut transaction = state.pool.begin().await?;
    lock_grow_log(&mut transaction, log_id).await?;
    validate_entry_order(&mut transaction, log_id, None, occurred_at, &payload.stage).await?;
    let entry = sqlx::query_as::<_, Entry>(
        "INSERT INTO grow_log_entries (grow_log_id, occurred_at, stage, description)
         VALUES ($1, $2, $3, $4)
         RETURNING id, grow_log_id, occurred_at, stage, description, inserted_at, updated_at",
    )
    .bind(log_id)
    .bind(occurred_at.to_sqlx())
    .bind(&payload.stage)
    .bind(payload.description)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
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
    let existing_entry =
        crate::grow_logs::find_entry(&state.pool, authenticated_user.id, entry_id).await?;
    let payload = decode_json_request(payload)?;
    let occurred_at = parse_timestamp("occurred_at", &payload.occurred_at)?;
    validate_stage(&payload.stage)?;
    let mut transaction = state.pool.begin().await?;
    lock_grow_log(&mut transaction, existing_entry.grow_log_id).await?;
    validate_entry_order(
        &mut transaction,
        existing_entry.grow_log_id,
        Some(entry_id),
        occurred_at,
        &payload.stage,
    )
    .await?;
    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE grow_log_entries
         SET occurred_at = $1, stage = $2, description = $3, updated_at = NOW()
         WHERE id = $4
         RETURNING id, grow_log_id, occurred_at, stage, description, inserted_at, updated_at",
    )
    .bind(occurred_at.to_sqlx())
    .bind(payload.stage)
    .bind(payload.description)
    .bind(entry_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    let photos = sqlx::query_as::<_, PhotoRow>(
        "SELECT id, media_type, original_filename, inserted_at, updated_at
         FROM grow_log_entry_photos WHERE grow_log_entry_id = $1 ORDER BY id",
    )
    .bind(entry_id)
    .fetch_all(&mut *transaction)
    .await?
    .into_iter()
    .map(PhotoRow::into_photo)
    .collect();
    transaction.commit().await?;
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

async fn lock_grow_log(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    log_id: i64,
) -> Result<(), AppError> {
    sqlx::query("SELECT id FROM grow_logs WHERE id = $1 FOR UPDATE")
        .bind(log_id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(())
}

async fn validate_entry_order(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    log_id: i64,
    entry_id: Option<i64>,
    occurred_at: Timestamp,
    stage: &str,
) -> Result<(), AppError> {
    let conflicting_date: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM grow_log_entries
            WHERE grow_log_id = $1 AND ($2::bigint IS NULL OR id <> $2)
              AND (occurred_at AT TIME ZONE 'UTC')::date = ($3::timestamptz AT TIME ZONE 'UTC')::date
        )",
    )
    .bind(log_id)
    .bind(entry_id)
    .bind(occurred_at.to_sqlx())
    .fetch_one(&mut **transaction)
    .await?;
    if conflicting_date {
        return Err(AppError::Validation(vec![FieldError {
            field: "occurred_at".to_owned(),
            message: "an entry already exists for this date".to_owned(),
        }]));
    }

    let conflicting_stage: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM grow_log_entries
            WHERE grow_log_id = $1 AND ($2::bigint IS NULL OR id <> $2)
              AND ((occurred_at < $3 AND array_position($4::text[], stage) > array_position($4::text[], $5::text))
                OR (occurred_at > $3 AND array_position($4::text[], stage) < array_position($4::text[], $5::text)))
        )",
    )
    .bind(log_id)
    .bind(entry_id)
    .bind(occurred_at.to_sqlx())
    .bind(&VALID_STAGES[..])
    .bind(stage)
    .fetch_one(&mut **transaction)
    .await?;
    if conflicting_stage {
        return Err(AppError::Validation(vec![FieldError {
            field: "stage".to_owned(),
            message: "must follow the order of the other entries".to_owned(),
        }]));
    }
    Ok(())
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
    inserted_at: jiff_sqlx::Timestamp,
    updated_at: jiff_sqlx::Timestamp,
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
