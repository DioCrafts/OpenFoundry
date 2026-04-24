use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub window_type: String,
    pub duration_seconds: i32,
    pub slide_seconds: i32,
    pub session_gap_seconds: i32,
    pub allowed_lateness_seconds: i32,
    pub aggregation_keys: Vec<String>,
    pub measure_fields: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWindowRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub window_type: Option<String>,
    pub duration_seconds: Option<i32>,
    pub slide_seconds: Option<i32>,
    pub session_gap_seconds: Option<i32>,
    pub allowed_lateness_seconds: Option<i32>,
    pub aggregation_keys: Vec<String>,
    pub measure_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWindowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub window_type: Option<String>,
    pub duration_seconds: Option<i32>,
    pub slide_seconds: Option<i32>,
    pub session_gap_seconds: Option<i32>,
    pub allowed_lateness_seconds: Option<i32>,
    pub aggregation_keys: Option<Vec<String>>,
    pub measure_fields: Option<Vec<String>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WindowRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub window_type: String,
    pub duration_seconds: i32,
    pub slide_seconds: i32,
    pub session_gap_seconds: i32,
    pub allowed_lateness_seconds: i32,
    pub aggregation_keys: SqlJson<Vec<String>>,
    pub measure_fields: SqlJson<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<WindowRow> for WindowDefinition {
    fn from(value: WindowRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            window_type: value.window_type,
            duration_seconds: value.duration_seconds,
            slide_seconds: value.slide_seconds,
            session_gap_seconds: value.session_gap_seconds,
            allowed_lateness_seconds: value.allowed_lateness_seconds,
            aggregation_keys: value.aggregation_keys.0,
            measure_fields: value.measure_fields.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
