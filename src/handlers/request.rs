use axum::Json;
use axum::extract::rejection::JsonRejection;
use serde::de::DeserializeOwned;

use crate::error::{AppError, FieldError};

pub fn decode_json_request<T: DeserializeOwned>(
    request: Result<Json<T>, JsonRejection>,
) -> Result<T, AppError> {
    request.map(|Json(payload)| payload).map_err(|_| {
        AppError::Validation(vec![FieldError {
            field: "request".to_owned(),
            message: "request body is invalid".to_owned(),
        }])
    })
}
