use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::decode_json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationProvider {
    Github,
    Gitlab,
}

impl IntegrationProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Github => "GitHub",
            Self::Gitlab => "GitLab",
        }
    }
}

impl Display for IntegrationProvider {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for IntegrationProvider {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "github" => Ok(Self::Github),
            "gitlab" => Ok(Self::Gitlab),
            _ => Err(format!("unsupported integration provider: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIntegration {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub provider: IntegrationProvider,
    pub external_namespace: String,
    pub external_project: String,
    pub external_url: String,
    pub sync_mode: String,
    pub ci_trigger_strategy: String,
    pub status: String,
    pub default_branch: String,
    pub branch_mapping: Vec<String>,
    pub webhook_url: String,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSyncRun {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub repository_id: Uuid,
    pub trigger: String,
    pub status: String,
    pub commit_sha: String,
    pub branch_name: String,
    pub summary: String,
    pub checks: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationDetail {
    pub integration: RepositoryIntegration,
    pub sync_runs: Vec<ExternalSyncRun>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateIntegrationRequest {
    pub repository_id: Uuid,
    pub provider: IntegrationProvider,
    pub external_namespace: String,
    pub external_project: String,
    pub external_url: String,
    pub sync_mode: String,
    pub ci_trigger_strategy: String,
    pub default_branch: String,
    #[serde(default)]
    pub branch_mapping: Vec<String>,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateIntegrationRequest {
    pub external_namespace: Option<String>,
    pub external_project: Option<String>,
    pub external_url: Option<String>,
    pub sync_mode: Option<String>,
    pub ci_trigger_strategy: Option<String>,
    pub status: Option<String>,
    pub default_branch: Option<String>,
    pub branch_mapping: Option<Vec<String>>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerSyncRequest {
    pub trigger: String,
    pub commit_sha: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IntegrationRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub provider: String,
    pub external_namespace: String,
    pub external_project: String,
    pub external_url: String,
    pub sync_mode: String,
    pub ci_trigger_strategy: String,
    pub status: String,
    pub default_branch: String,
    pub branch_mapping: Value,
    pub webhook_url: String,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncRunRow {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub repository_id: Uuid,
    pub trigger: String,
    pub status: String,
    pub commit_sha: String,
    pub branch_name: String,
    pub summary: String,
    pub checks: Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl TryFrom<IntegrationRow> for RepositoryIntegration {
    type Error = String;

    fn try_from(row: IntegrationRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            provider: IntegrationProvider::from_str(&row.provider)?,
            external_namespace: row.external_namespace,
            external_project: row.external_project,
            external_url: row.external_url,
            sync_mode: row.sync_mode,
            ci_trigger_strategy: row.ci_trigger_strategy,
            status: row.status,
            default_branch: row.default_branch,
            branch_mapping: decode_json(row.branch_mapping, "branch_mapping")?,
            webhook_url: row.webhook_url,
            last_synced_at: row.last_synced_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<SyncRunRow> for ExternalSyncRun {
    type Error = String;

    fn try_from(row: SyncRunRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            integration_id: row.integration_id,
            repository_id: row.repository_id,
            trigger: row.trigger,
            status: row.status,
            commit_sha: row.commit_sha,
            branch_name: row.branch_name,
            summary: row.summary,
            checks: decode_json(row.checks, "checks")?,
            started_at: row.started_at,
            completed_at: row.completed_at,
        })
    }
}
