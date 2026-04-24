use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{data_classification::ClassificationLevel, decode_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPolicy {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub scope: String,
    pub classification: ClassificationLevel,
    pub retention_days: i32,
    pub legal_hold: bool,
    pub purge_mode: String,
    pub active: bool,
    pub rules: Vec<String>,
    pub updated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub scope: String,
    pub classification: ClassificationLevel,
    pub retention_days: i32,
    #[serde(default)]
    pub legal_hold: bool,
    pub purge_mode: String,
    #[serde(default = "default_active")]
    pub active: bool,
    #[serde(default)]
    pub rules: Vec<String>,
    pub updated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub scope: Option<String>,
    pub classification: Option<ClassificationLevel>,
    pub retention_days: Option<i32>,
    pub legal_hold: Option<bool>,
    pub purge_mode: Option<String>,
    pub active: Option<bool>,
    pub rules: Option<Vec<String>>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorStatus {
    pub service_name: String,
    pub subject: String,
    pub connected: bool,
    pub last_event_at: Option<DateTime<Utc>>,
    pub backlog_depth: i32,
    pub health: String,
    pub next_pull_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PolicyRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub scope: String,
    pub classification: String,
    pub retention_days: i32,
    pub legal_hold: bool,
    pub purge_mode: String,
    pub active: bool,
    pub rules: Value,
    pub updated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<PolicyRow> for AuditPolicy {
    type Error = String;

    fn try_from(row: PolicyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            scope: row.scope,
            classification: std::str::FromStr::from_str(&row.classification)?,
            retention_days: row.retention_days,
            legal_hold: row.legal_hold,
            purge_mode: row.purge_mode,
            active: row.active,
            rules: decode_json(row.rules, "rules")?,
            updated_by: row.updated_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn default_active() -> bool {
    true
}
