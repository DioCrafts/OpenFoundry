use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetView {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub name: String,
    pub description: String,
    pub sql_text: String,
    pub source_branch: Option<String>,
    pub source_version: Option<i32>,
    pub materialized: bool,
    pub refresh_on_source_update: bool,
    pub format: String,
    pub current_version: i32,
    pub storage_path: Option<String>,
    pub row_count: i64,
    pub schema_fields: serde_json::Value,
    pub last_refreshed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDatasetViewRequest {
    pub name: String,
    pub description: Option<String>,
    pub sql: String,
    pub source_branch: Option<String>,
    pub source_version: Option<i32>,
    pub materialized: Option<bool>,
    pub refresh_on_source_update: Option<bool>,
}
