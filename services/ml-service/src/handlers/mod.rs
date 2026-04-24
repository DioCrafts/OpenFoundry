pub mod deployments;
pub mod experiments;
pub mod features;
pub mod models;
pub mod overview;
pub mod predictions;
pub mod training;

use axum::{Json, http::StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("ml-service database error: {cause}");
    internal_error("database operation failed")
}

pub fn deserialize_json<T>(value: Value) -> T
where
    T: DeserializeOwned + Default,
{
    serde_json::from_value(value).unwrap_or_default()
}

pub fn deserialize_optional_json<T>(value: Option<Value>) -> Option<T>
where
    T: DeserializeOwned,
{
    value.and_then(|inner| serde_json::from_value(inner).ok())
}

pub fn to_json<T>(value: &T) -> Value
where
    T: Serialize,
{
    serde_json::to_value(value).unwrap_or_else(|_| json!(null))
}
