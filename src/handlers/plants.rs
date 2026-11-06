use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};
use crate::grow_logs::{Plant, PlantDetail, list_plant_logs, validate_nonempty_field};
use crate::handlers::request::decode_json_request;

#[derive(Debug, Deserialize)]
pub struct PlantPayload {
    pub name: String,
}

pub async fn index(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<Plant>>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let plants = sqlx::query_as::<_, Plant>(
        "SELECT id, name, inserted_at, updated_at
         FROM plants
         WHERE user_id = $1
         ORDER BY name, id",
    )
    .bind(authenticated_user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(plants))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    payload: Result<Json<PlantPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Plant>), AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload = decode_json_request(payload)?;
    validate_nonempty_field("name", &payload.name)?;
    let plant = sqlx::query_as::<_, Plant>(
        "INSERT INTO plants (user_id, name)
         VALUES ($1, $2)
         RETURNING id, name, inserted_at, updated_at",
    )
    .bind(authenticated_user.id)
    .bind(payload.name.trim())
    .fetch_one(&state.pool)
    .await
    .map_err(map_plant_name_conflict)?;
    Ok((StatusCode::CREATED, Json(plant)))
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(plant_id): Path<i64>,
) -> Result<Json<PlantDetail>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let plant = crate::grow_logs::find_plant(&state.pool, authenticated_user.id, plant_id).await?;
    let grow_logs = list_plant_logs(&state.pool, authenticated_user.id, plant_id).await?;
    Ok(Json(PlantDetail { plant, grow_logs }))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(plant_id): Path<i64>,
    payload: Result<Json<PlantPayload>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Plant>, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload = decode_json_request(payload)?;
    validate_nonempty_field("name", &payload.name)?;
    crate::grow_logs::find_plant(&state.pool, authenticated_user.id, plant_id).await?;
    let plant = sqlx::query_as::<_, Plant>(
        "UPDATE plants
         SET name = $1, updated_at = NOW()
         WHERE id = $2 AND user_id = $3
         RETURNING id, name, inserted_at, updated_at",
    )
    .bind(payload.name.trim())
    .bind(plant_id)
    .bind(authenticated_user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(map_plant_name_conflict)?
    .ok_or(AppError::NotFound)?;
    Ok(Json(plant))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(plant_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let authenticated_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let mut transaction = state.pool.begin().await?;
    let locked_plant_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM plants WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(plant_id)
    .bind(authenticated_user.id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    let referenced_log_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM grow_logs WHERE plant_id = $1)")
            .bind(locked_plant_id)
            .fetch_one(&mut *transaction)
            .await?;
    if referenced_log_exists {
        return Err(AppError::Conflict);
    }
    sqlx::query("DELETE FROM plants WHERE id = $1 AND user_id = $2")
        .bind(plant_id)
        .bind(authenticated_user.id)
        .execute(&mut *transaction)
        .await
        .map_err(map_plant_delete_error)?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn map_plant_name_conflict(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.is_unique_violation()
    {
        return AppError::Validation(vec![FieldError {
            field: "name".to_owned(),
            message: "plant name is already in use".to_owned(),
        }]);
    }
    error.into()
}

fn map_plant_delete_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23503")
    {
        return AppError::Conflict;
    }
    error.into()
}
