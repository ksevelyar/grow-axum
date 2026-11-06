use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;
use crate::users::User;

const SESSION_DURATION_SECONDS: u64 = 365 * 24 * 60 * 60;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub exp: u64,
    pub email: String,
}

fn read_jwt_secret() -> Result<String, AppError> {
    std::env::var("JWT_SECRET").map_err(|_| AppError::Configuration("JWT_SECRET is not set".into()))
}

pub fn encode_session_token(email: String) -> Result<String, AppError> {
    let jwt_secret = read_jwt_secret()?;
    let expiry = Utc::now().timestamp() as u64 + SESSION_DURATION_SECONDS;
    let claims = Claims { exp: expiry, email };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|error| AppError::Application(error.to_string()))
}

pub fn decode_session_token(session_token: &str) -> Result<TokenData<Claims>, AppError> {
    let jwt_secret = read_jwt_secret()?;
    decode(
        session_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|error| AppError::Application(error.to_string()))
}

pub fn build_session_cookie<'cookie>(session_token: String) -> Cookie<'cookie> {
    Cookie::build(("jwt", session_token))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::seconds(
            SESSION_DURATION_SECONDS as i64,
        ))
        .secure(!cfg!(debug_assertions))
        .build()
}

pub async fn authenticate_cookie(pool: &PgPool, cookie_jar: &CookieJar) -> Result<User, AppError> {
    let session_token = cookie_jar.get("jwt").ok_or(AppError::Unauthorized)?.value();
    let session_claims = decode_session_token(session_token)?;
    crate::users::find_by_email(pool, &session_claims.claims.email).await
}
