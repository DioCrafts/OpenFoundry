use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::decode_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDefinition {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub branch_name: String,
    pub sha: String,
    pub parent_sha: Option<String>,
    pub title: String,
    pub description: String,
    pub author_name: String,
    pub author_email: String,
    pub files_changed: i32,
    pub additions: i32,
    pub deletions: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommitRequest {
    pub branch_name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub author_name: String,
    #[serde(default = "default_additions")]
    pub additions: i32,
    #[serde(default = "default_deletions")]
    pub deletions: i32,
    #[serde(default)]
    pub files: Vec<CommitFileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileChange {
    pub path: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRun {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub branch_name: String,
    pub commit_sha: String,
    pub pipeline_name: String,
    pub status: String,
    pub trigger: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCiRunRequest {
    pub branch_name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommitRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub branch_name: String,
    pub sha: String,
    pub parent_sha: Option<String>,
    pub title: String,
    pub description: String,
    pub author_name: String,
    pub author_email: String,
    pub files_changed: i32,
    pub additions: i32,
    pub deletions: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CiRunRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub branch_name: String,
    pub commit_sha: String,
    pub pipeline_name: String,
    pub status: String,
    pub trigger: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub checks: Value,
}

impl TryFrom<CommitRow> for CommitDefinition {
    type Error = String;

    fn try_from(row: CommitRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            branch_name: row.branch_name,
            sha: row.sha,
            parent_sha: row.parent_sha,
            title: row.title,
            description: row.description,
            author_name: row.author_name,
            author_email: row.author_email,
            files_changed: row.files_changed,
            additions: row.additions,
            deletions: row.deletions,
            created_at: row.created_at,
        })
    }
}

impl TryFrom<CiRunRow> for CiRun {
    type Error = String;

    fn try_from(row: CiRunRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            branch_name: row.branch_name,
            commit_sha: row.commit_sha,
            pipeline_name: row.pipeline_name,
            status: row.status,
            trigger: row.trigger,
            started_at: row.started_at,
            completed_at: row.completed_at,
            checks: decode_json(row.checks, "checks")?,
        })
    }
}

fn default_additions() -> i32 {
    14
}
fn default_deletions() -> i32 {
    3
}
