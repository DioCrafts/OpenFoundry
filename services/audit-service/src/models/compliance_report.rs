use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{audit_event::AuditEvent, decode_json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStandard {
    Soc2,
    Iso27001,
    Hipaa,
    Gdpr,
    Itar,
}

impl ComplianceStandard {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Soc2 => "soc2",
            Self::Iso27001 => "iso27001",
            Self::Hipaa => "hipaa",
            Self::Gdpr => "gdpr",
            Self::Itar => "itar",
        }
    }
}

impl FromStr for ComplianceStandard {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "soc2" => Ok(Self::Soc2),
            "iso27001" => Ok(Self::Iso27001),
            "hipaa" => Ok(Self::Hipaa),
            "gdpr" => Ok(Self::Gdpr),
            "itar" => Ok(Self::Itar),
            _ => Err(format!("unsupported compliance standard: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub control_id: String,
    pub title: String,
    pub status: String,
    pub evidence: String,
}

impl ComplianceFinding {
    pub fn new(control_id: &str, title: &str, status: &str, evidence: &str) -> Self {
        Self {
            control_id: control_id.to_string(),
            title: title.to_string(),
            status: status.to_string(),
            evidence: evidence.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceArtifact {
    pub file_name: String,
    pub mime_type: String,
    pub storage_url: String,
    pub checksum: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: uuid::Uuid,
    pub standard: ComplianceStandard,
    pub title: String,
    pub scope: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub status: String,
    pub findings: Vec<ComplianceFinding>,
    pub artifact: ComplianceArtifact,
    pub relevant_event_count: i64,
    pub policy_count: i64,
    pub control_summary: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReportRequest {
    pub standard: ComplianceStandard,
    pub title: String,
    pub scope: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdprExportRequest {
    pub subject_id: String,
    #[serde(default = "default_portable_format")]
    pub portable_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdprExportPayload {
    pub subject_id: String,
    pub generated_at: DateTime<Utc>,
    pub portable_format: String,
    pub event_count: usize,
    pub resources: Vec<String>,
    pub audit_excerpt: Vec<AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdprEraseRequest {
    pub subject_id: String,
    #[serde(default)]
    pub hard_delete: bool,
    #[serde(default)]
    pub legal_hold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdprEraseResponse {
    pub subject_id: String,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub masked_event_count: usize,
    pub affected_resources: Vec<String>,
    pub legal_hold: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ComplianceReportRow {
    pub id: uuid::Uuid,
    pub standard: String,
    pub title: String,
    pub scope: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub status: String,
    pub findings: Value,
    pub artifact: Value,
    pub relevant_event_count: i64,
    pub policy_count: i64,
    pub control_summary: String,
    pub expires_at: DateTime<Utc>,
}

impl TryFrom<ComplianceReportRow> for ComplianceReport {
    type Error = String;

    fn try_from(row: ComplianceReportRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            standard: ComplianceStandard::from_str(&row.standard)?,
            title: row.title,
            scope: row.scope,
            window_start: row.window_start,
            window_end: row.window_end,
            generated_at: row.generated_at,
            status: row.status,
            findings: decode_json(row.findings, "findings")?,
            artifact: decode_json(row.artifact, "artifact")?,
            relevant_event_count: row.relevant_event_count,
            policy_count: row.policy_count,
            control_summary: row.control_summary,
            expires_at: row.expires_at,
        })
    }
}

fn default_portable_format() -> String {
    "json".to_string()
}
