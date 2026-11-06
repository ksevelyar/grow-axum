use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    Validation(Vec<FieldError>),
    Application(String),
    Configuration(String),
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::Application(error.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(formatter, "unauthorized"),
            Self::Validation(field_errors) => {
                write!(formatter, "validation failed: {field_errors:?}")
            }
            Self::Application(message) => write!(formatter, "application error: {message}"),
            Self::Configuration(message) => write!(formatter, "configuration error: {message}"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, response_body) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"})),
            Self::Validation(field_errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "validation failed", "fields": field_errors}),
            ),
            Self::Application(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "internal error"}),
            ),
            Self::Configuration(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, json!({"error": message}))
            }
        };
        (status_code, Json(response_body)).into_response()
    }
}
