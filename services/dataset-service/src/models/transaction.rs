use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetTransaction {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub view_id: Option<Uuid>,
    pub operation: String,
    pub branch_name: Option<String>,
    pub status: String,
    pub summary: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
}
