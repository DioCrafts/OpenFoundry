use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{data_classification::ClassificationLevel, decode_json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventStatus {
    Success,
    Failure,
    Denied,
}

impl AuditEventStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Denied => "denied",
        }
    }
}

impl FromStr for AuditEventStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "success" => Ok(Self::Success),
            "failure" => Ok(Self::Failure),
            "denied" => Ok(Self::Denied),
            _ => Err(format!("unsupported audit event status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn is_critical(self) -> bool {
        matches!(self, Self::Critical)
    }
}

impl FromStr for AuditSeverity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("unsupported audit severity: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: uuid::Uuid,
    pub sequence: i64,
    pub previous_hash: String,
    pub entry_hash: String,
    pub source_service: String,
    pub channel: String,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub status: AuditEventStatus,
    pub severity: AuditSeverity,
    pub classification: ClassificationLevel,
    pub subject_id: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub metadata: Value,
    pub labels: Vec<String>,
    pub retention_until: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendAuditEventRequest {
    pub source_service: String,
    pub channel: String,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub status: AuditEventStatus,
    pub severity: AuditSeverity,
    pub classification: ClassificationLevel,
    pub subject_id: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default = "default_retention_days")]
    pub retention_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditOverview {
    pub event_count: i64,
    pub critical_event_count: i64,
    pub collector_count: i64,
    pub active_policy_count: i64,
    pub anomaly_count: i64,
    pub gdpr_subject_count: i64,
    pub latest_event: Option<AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventListResponse {
    pub items: Vec<AuditEvent>,
    pub anomalies: Vec<crate::models::data_classification::AnomalyAlert>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditEventRow {
    pub id: uuid::Uuid,
    pub sequence: i64,
    pub previous_hash: String,
    pub entry_hash: String,
    pub source_service: String,
    pub channel: String,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub status: String,
    pub severity: String,
    pub classification: String,
    pub subject_id: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub metadata: Value,
    pub labels: Value,
    pub retention_until: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
}

impl TryFrom<AuditEventRow> for AuditEvent {
    type Error = String;

    fn try_from(row: AuditEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            sequence: row.sequence,
            previous_hash: row.previous_hash,
            entry_hash: row.entry_hash,
            source_service: row.source_service,
            channel: row.channel,
            actor: row.actor,
            action: row.action,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            status: AuditEventStatus::from_str(&row.status)?,
            severity: AuditSeverity::from_str(&row.severity)?,
            classification: ClassificationLevel::from_str(&row.classification)?,
            subject_id: row.subject_id,
            ip_address: row.ip_address,
            location: row.location,
            metadata: row.metadata,
            labels: decode_json(row.labels, "labels")?,
            retention_until: row.retention_until,
            occurred_at: row.occurred_at,
            ingested_at: row.ingested_at,
        })
    }
}

fn default_retention_days() -> i32 {
    365
}
