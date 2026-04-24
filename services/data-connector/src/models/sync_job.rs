use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub target_dataset_id: Option<Uuid>,
    pub table_name: String,
    pub status: String,
    pub rows_synced: i64,
    pub error: Option<String>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub scheduled_at: DateTime<Utc>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub result_dataset_version: Option<i32>,
    pub sync_metadata: serde_json::Value,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub table_name: String,
    pub target_dataset_id: Option<Uuid>,
    pub schedule_at: Option<DateTime<Utc>>,
    pub max_attempts: Option<i32>,
}
