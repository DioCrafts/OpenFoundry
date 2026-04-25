use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{comment::ReviewComment, commit::CiRun, decode_json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeRequestStatus {
    Open,
    Approved,
    Merged,
    Closed,
}

impl MergeRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Approved => "approved",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }
}

impl FromStr for MergeRequestStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "approved" => Ok(Self::Approved),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            _ => Err(format!("unsupported merge request status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewerState {
    pub reviewer: String,
    pub approved: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequestDefinition {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub status: MergeRequestStatus,
    pub author: String,
    pub labels: Vec<String>,
    pub reviewers: Vec<ReviewerState>,
    pub approvals_required: i32,
    pub changed_files: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequestDetail {
    pub merge_request: MergeRequestDefinition,
    pub comments: Vec<ReviewComment>,
    pub approval_count: usize,
    pub thread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeMergeRequestRequest {
    pub merged_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequestMergeResult {
    pub merge_request: MergeRequestDefinition,
    pub merge_commit_sha: String,
    pub target_branch: String,
    pub ci_run: Option<CiRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMergeRequestRequest {
    pub repository_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub reviewers: Vec<ReviewerState>,
    #[serde(default = "default_approvals_required")]
    pub approvals_required: i32,
    #[serde(default = "default_changed_files")]
    pub changed_files: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMergeRequestRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<MergeRequestStatus>,
    pub labels: Option<Vec<String>>,
    pub reviewers: Option<Vec<ReviewerState>>,
    pub approvals_required: Option<i32>,
    pub changed_files: Option<i32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MergeRequestRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub status: String,
    pub author: String,
    pub labels: Value,
    pub reviewers: Value,
    pub approvals_required: i32,
    pub changed_files: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
}

impl TryFrom<MergeRequestRow> for MergeRequestDefinition {
    type Error = String;

    fn try_from(row: MergeRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            title: row.title,
            description: row.description,
            source_branch: row.source_branch,
            target_branch: row.target_branch,
            status: MergeRequestStatus::from_str(&row.status)?,
            author: row.author,
            labels: decode_json(row.labels, "labels")?,
            reviewers: decode_json(row.reviewers, "reviewers")?,
            approvals_required: row.approvals_required,
            changed_files: row.changed_files,
            created_at: row.created_at,
            updated_at: row.updated_at,
            merged_at: row.merged_at,
        })
    }
}

fn default_approvals_required() -> i32 {
    2
}
fn default_changed_files() -> i32 {
    6
}
