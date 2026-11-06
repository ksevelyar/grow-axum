use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};
use crate::grow_logs::{Photo, find_photo, remove_photo_files};

pub async fn create(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(entry_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Photo>), AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::grow_logs::find_entry(&state.pool, authenticated_user.id, entry_id).await?;
    let file_field = multipart
        .next_field()
        .await
        .map_err(|_| invalid_photo_upload())?
        .ok_or_else(invalid_photo_upload)?;
    let original_filename = file_field.file_name().map(str::to_owned);
    let media_type = file_field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_owned();
    let file_bytes = file_field
        .bytes()
        .await
        .map_err(|_| invalid_photo_upload())?;
    if multipart
        .next_field()
        .await
        .map_err(|_| invalid_photo_upload())?
        .is_some()
    {
        return Err(invalid_photo_upload());
    }

    tokio::fs::create_dir_all(&state.photo_storage_directory)
        .await
        .map_err(|error| AppError::Application(error.to_string()))?;
    let storage_key = uuid::Uuid::new_v4().to_string();
    let storage_path = state.photo_storage_directory.join(&storage_key);
    tokio::fs::write(&storage_path, file_bytes)
        .await
        .map_err(|error| AppError::Application(error.to_string()))?;

    let inserted_photo = sqlx::query_as::<_, PhotoRow>(
        "INSERT INTO grow_log_entry_photos
            (grow_log_entry_id, storage_key, media_type, original_filename)
         VALUES ($1, $2, $3, $4)
         RETURNING id, media_type, original_filename, inserted_at, updated_at",
    )
    .bind(entry_id)
    .bind(&storage_key)
    .bind(media_type)
    .bind(original_filename)
    .fetch_one(&state.pool)
    .await;
    let inserted_photo = match inserted_photo {
        Ok(photo) => photo,
        Err(database_error) => {
            let _ = tokio::fs::remove_file(storage_path).await;
            return Err(database_error.into());
        }
    };
    Ok((StatusCode::CREATED, Json(inserted_photo.into_photo())))
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(photo_id): Path<i64>,
) -> Result<Response<Body>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let photo = find_photo(&state.pool, authenticated_user.id, photo_id).await?;
    let photo_path = state.photo_storage_directory.join(&photo.storage_key);
    let photo_bytes = tokio::fs::read(photo_path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound
        } else {
            AppError::Application(error.to_string())
        }
    })?;
    let media_type = HeaderValue::from_str(&photo.media_type)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, media_type)
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(photo_bytes))
        .map_err(|error| AppError::Application(error.to_string()))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(photo_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let photo = find_photo(&state.pool, authenticated_user.id, photo_id).await?;
    sqlx::query("DELETE FROM grow_log_entry_photos WHERE id = $1")
        .bind(photo_id)
        .execute(&state.pool)
        .await?;
    remove_photo_files(&state.photo_storage_directory, vec![photo.storage_key]).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn invalid_photo_upload() -> AppError {
    AppError::Validation(vec![FieldError {
        field: "file".to_owned(),
        message: "one file is required per upload".to_owned(),
    }])
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
        Photo {
            id: self.id,
            media_type: self.media_type,
            original_filename: self.original_filename,
            url: format!("/photos/{}", self.id),
            inserted_at: self.inserted_at,
            updated_at: self.updated_at,
        }
    }
}
