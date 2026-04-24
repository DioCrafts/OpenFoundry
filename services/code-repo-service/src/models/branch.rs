use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDefinition {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub head_sha: String,
    pub base_branch: Option<String>,
    pub is_default: bool,
    pub protected: bool,
    pub ahead_by: i32,
    pub pending_reviews: usize,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    #[serde(default)]
    pub protected: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BranchRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub head_sha: String,
    pub base_branch: Option<String>,
    pub is_default: bool,
    pub protected: bool,
    pub ahead_by: i32,
    pub pending_reviews: i32,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<BranchRow> for BranchDefinition {
    type Error = String;

    fn try_from(row: BranchRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            name: row.name,
            head_sha: row.head_sha,
            base_branch: row.base_branch,
            is_default: row.is_default,
            protected: row.protected,
            ahead_by: row.ahead_by,
            pending_reviews: row.pending_reviews as usize,
            updated_at: row.updated_at,
        })
    }
}

fn default_base_branch() -> String {
    "main".to_string()
}
