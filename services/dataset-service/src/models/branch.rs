use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetBranch {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub name: String,
    pub version: i32,
    pub base_version: i32,
    pub description: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDatasetBranchRequest {
    pub name: String,
    pub source_version: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MergeDatasetBranchRequest {
    pub target_branch: Option<String>,
}
