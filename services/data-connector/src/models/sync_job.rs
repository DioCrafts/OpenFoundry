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
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub table_name: String,
    pub target_dataset_id: Option<Uuid>,
}
