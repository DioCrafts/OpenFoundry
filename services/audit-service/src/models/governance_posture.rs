use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

use crate::models::{compliance_report::ComplianceStandard, decode_json};

#[derive(Debug, Clone, FromRow)]
pub struct GovernanceTemplateApplicationRow {
    pub id: uuid::Uuid,
    pub template_slug: String,
    pub template_name: String,
    pub scope: String,
    pub standards: Value,
    pub policy_names: Value,
    pub checkpoint_prompts: Value,
    pub sds_remediations: Value,
    pub default_report_standard: String,
    pub applied_by: String,
    pub applied_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceTemplateApplication {
    pub id: uuid::Uuid,
    pub template_slug: String,
    pub template_name: String,
    pub scope: String,
    pub standards: Vec<String>,
    pub policy_names: Vec<String>,
    pub checkpoint_prompts: Vec<String>,
    pub sds_remediations: Vec<String>,
    pub default_report_standard: ComplianceStandard,
    pub applied_by: String,
    pub applied_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<GovernanceTemplateApplicationRow> for GovernanceTemplateApplication {
    type Error = String;

    fn try_from(row: GovernanceTemplateApplicationRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            template_slug: row.template_slug,
            template_name: row.template_name,
            scope: row.scope,
            standards: decode_json(row.standards, "standards")?,
            policy_names: decode_json(row.policy_names, "policy_names")?,
            checkpoint_prompts: decode_json(row.checkpoint_prompts, "checkpoint_prompts")?,
            sds_remediations: decode_json(row.sds_remediations, "sds_remediations")?,
            default_report_standard: row.default_report_standard.parse()?,
            applied_by: row.applied_by,
            applied_at: row.applied_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePostureOverview {
    pub standards: Vec<CompliancePostureStandard>,
    pub supported_capabilities: Vec<String>,
    pub active_template_application_count: i64,
    pub active_legal_hold_policy_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePostureStandard {
    pub standard: ComplianceStandard,
    pub template_available: bool,
    pub applied_scope_count: i64,
    pub active_policy_count: i64,
    pub latest_report_status: Option<String>,
    pub latest_report_generated_at: Option<DateTime<Utc>>,
    pub coverage_score: i32,
    pub checkpoint_prompt_count: i64,
    pub sds_remediation_count: i64,
    pub evidence_summary: String,
}
