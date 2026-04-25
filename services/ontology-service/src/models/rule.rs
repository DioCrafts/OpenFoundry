use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleTriggerSpec {
    #[serde(default)]
    pub equals: HashMap<String, Value>,
    #[serde(default)]
    pub numeric_gte: HashMap<String, f64>,
    #[serde(default)]
    pub numeric_lte: HashMap<String, f64>,
    #[serde(default)]
    pub exists: Vec<String>,
    #[serde(default)]
    pub changed_properties: Vec<String>,
    #[serde(default)]
    pub markings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleScheduleSpec {
    pub property_name: String,
    pub offset_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAlertSpec {
    pub severity: String,
    pub title: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleEffectSpec {
    pub object_patch: Option<Value>,
    pub schedule: Option<RuleScheduleSpec>,
    pub alert: Option<RuleAlertSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RuleEvaluationMode {
    Advisory,
    Automatic,
}

impl std::fmt::Display for RuleEvaluationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Advisory => "advisory",
            Self::Automatic => "automatic",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct OntologyRuleRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub evaluation_mode: String,
    pub trigger_spec: Value,
    pub effect_spec: Value,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyRule {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub evaluation_mode: RuleEvaluationMode,
    pub trigger_spec: RuleTriggerSpec,
    pub effect_spec: RuleEffectSpec,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<OntologyRuleRow> for OntologyRule {
    type Error = serde_json::Error;

    fn try_from(row: OntologyRuleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            display_name: row.display_name,
            description: row.description,
            object_type_id: row.object_type_id,
            evaluation_mode: serde_json::from_value(Value::String(row.evaluation_mode))
                .unwrap_or(RuleEvaluationMode::Advisory),
            trigger_spec: serde_json::from_value(row.trigger_spec).unwrap_or_default(),
            effect_spec: serde_json::from_value(row.effect_spec).unwrap_or_default(),
            owner_id: row.owner_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub object_type_id: Uuid,
    pub evaluation_mode: Option<RuleEvaluationMode>,
    pub trigger_spec: Option<RuleTriggerSpec>,
    pub effect_spec: Option<RuleEffectSpec>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub evaluation_mode: Option<RuleEvaluationMode>,
    pub trigger_spec: Option<RuleTriggerSpec>,
    pub effect_spec: Option<RuleEffectSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ListRulesQuery {
    pub object_type_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListRulesResponse {
    pub data: Vec<OntologyRule>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct RuleEvaluateRequest {
    pub object_id: Uuid,
    #[serde(default)]
    pub properties_patch: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuleMatchResponse {
    pub rule_id: Uuid,
    pub matched: bool,
    pub trigger_payload: Value,
    pub effect_preview: Value,
}

#[derive(Debug, Serialize)]
pub struct RuleEvaluateResponse {
    pub rule: OntologyRule,
    pub matched: bool,
    pub trigger_payload: Value,
    pub effect_preview: Value,
    pub object: Value,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OntologyRuleRun {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub object_id: Uuid,
    pub matched: bool,
    pub simulated: bool,
    pub trigger_payload: Value,
    pub effect_preview: Option<Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MachineryInsight {
    pub rule_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub evaluation_mode: RuleEvaluationMode,
    pub matched_runs: usize,
    pub total_runs: usize,
    pub pending_schedules: usize,
    pub last_matched_at: Option<DateTime<Utc>>,
    pub last_object_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct MachineryInsightsResponse {
    pub object_type_id: Option<Uuid>,
    pub data: Vec<MachineryInsight>,
}
