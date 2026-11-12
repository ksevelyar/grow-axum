use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use std::path::Path;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Plant {
    pub id: i64,
    pub name: String,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GrowLog {
    pub id: i64,
    pub plant_id: i64,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub description: Option<String>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GrowLogSummary {
    pub id: i64,
    pub plant_id: i64,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub description: Option<String>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub thumbnail_url: Option<String>,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct GrowLogSummaryRow {
    id: i64,
    plant_id: i64,
    name: String,
    started_at: DateTime<Utc>,
    description: Option<String>,
    ended_at: Option<DateTime<Utc>>,
    outcome: Option<String>,
    thumbnail_photo_id: Option<i64>,
    inserted_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl GrowLogSummaryRow {
    fn into_grow_log_summary(self) -> GrowLogSummary {
        let thumbnail_url = self
            .thumbnail_photo_id
            .as_ref()
            .map(|photo_id| format!("/photos/{photo_id}/thumbnail"));

        GrowLogSummary {
            id: self.id,
            plant_id: self.plant_id,
            name: self.name,
            started_at: self.started_at,
            description: self.description,
            ended_at: self.ended_at,
            outcome: self.outcome,
            thumbnail_url,
            inserted_at: self.inserted_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Entry {
    pub id: i64,
    pub grow_log_id: i64,
    pub occurred_at: DateTime<Utc>,
    pub stage: String,
    pub description: Option<String>,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Photo {
    pub id: i64,
    pub media_type: String,
    pub original_filename: Option<String>,
    pub url: String,
    pub thumbnail_url: String,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct EntryWithPhotos {
    #[serde(flatten)]
    pub entry: Entry,
    pub photos: Vec<Photo>,
}

#[derive(Debug, Serialize)]
pub struct PlantDetail {
    #[serde(flatten)]
    pub plant: Plant,
    pub grow_logs: Vec<GrowLogSummary>,
}

#[derive(Debug, Serialize)]
pub struct GrowLogDetail {
    #[serde(flatten)]
    pub grow_log: GrowLog,
    pub entries: Vec<EntryWithPhotos>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OwnedPhoto {
    pub id: i64,
    pub grow_log_entry_id: i64,
    pub storage_key: String,
    pub media_type: String,
    pub original_filename: Option<String>,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn find_plant(pool: &PgPool, user_id: i64, plant_id: i64) -> Result<Plant, AppError> {
    sqlx::query_as::<_, Plant>(
        "SELECT id, name, inserted_at, updated_at
         FROM plants
         WHERE id = $1 AND user_id = $2",
    )
    .bind(plant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn find_log(pool: &PgPool, user_id: i64, log_id: i64) -> Result<GrowLog, AppError> {
    sqlx::query_as::<_, GrowLog>(
        "SELECT id, plant_id, name, started_at, description, ended_at, outcome, inserted_at, updated_at
         FROM grow_logs
         WHERE id = $1 AND user_id = $2",
    )
    .bind(log_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn find_entry(pool: &PgPool, user_id: i64, entry_id: i64) -> Result<Entry, AppError> {
    sqlx::query_as::<_, Entry>(
        "SELECT entry.id, entry.grow_log_id, entry.occurred_at, entry.stage,
                entry.description, entry.inserted_at, entry.updated_at
         FROM grow_log_entries entry
         JOIN grow_logs log ON log.id = entry.grow_log_id
         WHERE entry.id = $1 AND log.user_id = $2",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn find_photo(
    pool: &PgPool,
    user_id: i64,
    photo_id: i64,
) -> Result<OwnedPhoto, AppError> {
    sqlx::query_as::<_, OwnedPhoto>(
        "SELECT photo.id, photo.grow_log_entry_id, photo.storage_key, photo.media_type,
                photo.original_filename, photo.inserted_at, photo.updated_at
         FROM grow_log_entry_photos photo
         JOIN grow_log_entries entry ON entry.id = photo.grow_log_entry_id
         JOIN grow_logs log ON log.id = entry.grow_log_id
         JOIN plants plant ON plant.id = log.plant_id AND plant.user_id = log.user_id
         JOIN users owner ON owner.id = plant.user_id
         WHERE photo.id = $1 AND owner.id = $2",
    )
    .bind(photo_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn list_plant_logs(
    pool: &PgPool,
    user_id: i64,
    plant_id: i64,
) -> Result<Vec<GrowLogSummary>, AppError> {
    let log_summaries = sqlx::query_as::<_, GrowLogSummaryRow>(
        "SELECT log.id, log.plant_id, log.name, log.started_at, log.description,
                log.ended_at, log.outcome, newest_photo.id AS thumbnail_photo_id,
                log.inserted_at, log.updated_at
         FROM grow_logs log
         LEFT JOIN LATERAL (
             SELECT entry.occurred_at
             FROM grow_log_entries entry
             WHERE entry.grow_log_id = log.id
             ORDER BY entry.occurred_at DESC, entry.id DESC
             LIMIT 1
         ) latest_entry ON TRUE
         LEFT JOIN LATERAL (
             SELECT photo.id
             FROM grow_log_entries entry
             JOIN grow_log_entry_photos photo ON photo.grow_log_entry_id = entry.id
             WHERE entry.grow_log_id = log.id
             ORDER BY entry.occurred_at DESC, entry.id DESC, photo.id DESC
             LIMIT 1
         ) newest_photo ON TRUE
         WHERE log.user_id = $1 AND log.plant_id = $2
         ORDER BY latest_entry.occurred_at DESC NULLS LAST, log.id DESC",
    )
    .bind(user_id)
    .bind(plant_id)
    .fetch_all(pool)
    .await?;
    Ok(log_summaries
        .into_iter()
        .map(GrowLogSummaryRow::into_grow_log_summary)
        .collect())
}

pub async fn list_user_logs(pool: &PgPool, user_id: i64) -> Result<Vec<GrowLogSummary>, AppError> {
    let log_summaries = sqlx::query_as::<_, GrowLogSummaryRow>(
        "SELECT log.id, log.plant_id, log.name, log.started_at, log.description,
                log.ended_at, log.outcome, newest_photo.id AS thumbnail_photo_id,
                log.inserted_at, log.updated_at
         FROM grow_logs log
         LEFT JOIN LATERAL (
             SELECT entry.occurred_at
             FROM grow_log_entries entry
             WHERE entry.grow_log_id = log.id
             ORDER BY entry.occurred_at DESC, entry.id DESC
             LIMIT 1
         ) latest_entry ON TRUE
         LEFT JOIN LATERAL (
             SELECT photo.id
             FROM grow_log_entries entry
             JOIN grow_log_entry_photos photo ON photo.grow_log_entry_id = entry.id
             WHERE entry.grow_log_id = log.id
             ORDER BY entry.occurred_at DESC, entry.id DESC, photo.id DESC
             LIMIT 1
         ) newest_photo ON TRUE
         WHERE log.user_id = $1
         ORDER BY latest_entry.occurred_at DESC NULLS LAST, log.id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(log_summaries
        .into_iter()
        .map(GrowLogSummaryRow::into_grow_log_summary)
        .collect())
}

pub async fn list_entries(pool: &PgPool, log_id: i64) -> Result<Vec<EntryWithPhotos>, AppError> {
    let entries = sqlx::query_as::<_, Entry>(
        "SELECT id, grow_log_id, occurred_at, stage, description, inserted_at, updated_at
         FROM grow_log_entries
         WHERE grow_log_id = $1
         ORDER BY occurred_at DESC, id DESC",
    )
    .bind(log_id)
    .fetch_all(pool)
    .await?;

    let mut entries_with_photos = Vec::with_capacity(entries.len());
    for entry in entries {
        let photos = sqlx::query_as::<_, PhotoRow>(
            "SELECT id, media_type, original_filename, inserted_at, updated_at
             FROM grow_log_entry_photos
             WHERE grow_log_entry_id = $1
             ORDER BY id",
        )
        .bind(entry.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(PhotoRow::into_photo)
        .collect();
        entries_with_photos.push(EntryWithPhotos { entry, photos });
    }
    Ok(entries_with_photos)
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
    fn into_photo(self) -> Photo {
        let url = format!("/photos/{}", self.id);
        let thumbnail_url = format!("/photos/{}/thumbnail", self.id);
        Photo {
            id: self.id,
            media_type: self.media_type,
            original_filename: self.original_filename,
            url,
            thumbnail_url,
            inserted_at: self.inserted_at,
            updated_at: self.updated_at,
        }
    }
}

pub async fn find_log_detail(
    pool: &PgPool,
    user_id: i64,
    log_id: i64,
) -> Result<GrowLogDetail, AppError> {
    let grow_log = find_log(pool, user_id, log_id).await?;
    let entries = list_entries(pool, log_id).await?;
    Ok(GrowLogDetail { grow_log, entries })
}

pub async fn list_log_photo_storage_keys(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: i64,
    log_id: i64,
) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT photo.storage_key
         FROM grow_log_entry_photos photo
         JOIN grow_log_entries entry ON entry.id = photo.grow_log_entry_id
         JOIN grow_logs log ON log.id = entry.grow_log_id
         WHERE log.id = $1 AND log.user_id = $2",
    )
    .bind(log_id)
    .bind(user_id)
    .fetch_all(&mut **transaction)
    .await?)
}

pub async fn list_entry_photo_storage_keys(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: i64,
    entry_id: i64,
) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT photo.storage_key
         FROM grow_log_entry_photos photo
         JOIN grow_log_entries entry ON entry.id = photo.grow_log_entry_id
         JOIN grow_logs log ON log.id = entry.grow_log_id
         WHERE entry.id = $1 AND log.user_id = $2",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_all(&mut **transaction)
    .await?)
}

pub async fn remove_photo_files(
    photo_storage_directory: &Path,
    storage_keys: Vec<String>,
) -> Result<(), AppError> {
    for storage_key in storage_keys {
        let thumbnail_storage_key = derive_photo_thumbnail_storage_key(&storage_key);
        for photo_storage_key in [storage_key, thumbnail_storage_key] {
            let photo_path = photo_storage_directory.join(photo_storage_key);
            match tokio::fs::remove_file(photo_path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::Application(error.to_string())),
            }
        }
    }
    Ok(())
}

pub fn derive_photo_thumbnail_storage_key(storage_key: &str) -> String {
    format!("{storage_key}.thumbnail.jpg")
}

pub fn validate_nonempty_field(field_name: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(vec![crate::error::FieldError {
            field: field_name.to_owned(),
            message: "must not be empty".to_owned(),
        }]));
    }
    Ok(())
}

pub fn parse_timestamp(field_name: &str, value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| {
            AppError::Validation(vec![crate::error::FieldError {
                field: field_name.to_owned(),
                message: "must be a valid timestamp".to_owned(),
            }])
        })
}

pub fn validate_date_order(
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    if ended_at.is_some_and(|end_timestamp| end_timestamp < started_at) {
        return Err(AppError::Validation(vec![crate::error::FieldError {
            field: "ended_at".to_owned(),
            message: "must not precede started_at".to_owned(),
        }]));
    }
    Ok(())
}
