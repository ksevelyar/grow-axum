use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::authentication::hash_password;
use crate::error::{AppError, FieldError};

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    pub email: String,
    pub handle: String,
    pub password: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionPayload {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub timezone: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct BackendUser {
    pub email: String,
    pub password_hash: String,
}

pub async fn create(pool: &PgPool, payload: CreateUserPayload) -> Result<User, AppError> {
    payload.timezone.parse::<chrono_tz::Tz>().map_err(|_| {
        AppError::Validation(vec![FieldError {
            field: "timezone".into(),
            message: "unknown timezone".into(),
        }])
    })?;
    let password_hash = hash_password(&payload.password)?;

    sqlx::query_as::<_, User>(
        "INSERT INTO users (email, handle, password_hash, timezone)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, timezone",
    )
    .bind(payload.email)
    .bind(payload.handle)
    .bind(password_hash)
    .bind(payload.timezone)
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            AppError::Validation(vec![FieldError {
                field: "email".into(),
                message: "email is already taken".into(),
            }])
        }
        error => error.into(),
    })
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT id, email, timezone FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)
}

pub async fn find_backend_by_email(pool: &PgPool, email: &str) -> Result<BackendUser, AppError> {
    sqlx::query_as::<_, BackendUser>("SELECT email, password_hash FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)
}
