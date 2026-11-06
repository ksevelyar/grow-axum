use axum::response::IntoResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::{authenticate_cookie, build_session_cookie, encode_session_token};
use crate::error::AppError;
use crate::users::{CreateUserPayload, User};

#[derive(Deserialize)]
pub struct CreateSessionPayload {
    pub email: String,
    pub password: String,
}

pub async fn show_current_user(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<User>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    Ok(Json(user))
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(payload): Json<CreateSessionPayload>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let backend_user = crate::users::find_backend_by_email(&state.pool, &payload.email).await?;
    let password_is_valid =
        bcrypt::verify(&payload.password, &backend_user.password_hash).unwrap_or(false);
    if !password_is_valid {
        return Err(AppError::Unauthorized);
    }

    let session_token = encode_session_token(backend_user.email)?;
    Ok((
        StatusCode::CREATED,
        cookie_jar.add(build_session_cookie(session_token)),
    ))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = crate::users::create(&state.pool, payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}
