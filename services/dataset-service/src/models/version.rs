use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetVersion {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub version: i32,
    pub message: String,
    pub size_bytes: i64,
    pub row_count: i64,
    pub storage_path: String,
    pub transaction_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
