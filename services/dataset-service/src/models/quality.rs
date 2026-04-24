use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetValueCount {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetColumnProfile {
    pub name: String,
    pub field_type: String,
    pub nullable: bool,
    pub null_count: i64,
    pub null_rate: f64,
    pub distinct_count: i64,
    pub uniqueness_rate: f64,
    pub sample_values: Vec<DatasetValueCount>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub average_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRuleResult {
    pub rule_id: Uuid,
    pub name: String,
    pub rule_type: String,
    pub severity: String,
    pub passed: bool,
    pub measured_value: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetQualityProfile {
    pub row_count: i64,
    pub column_count: i64,
    pub duplicate_rows: i64,
    pub completeness_ratio: f64,
    pub uniqueness_ratio: f64,
    pub generated_at: DateTime<Utc>,
    pub columns: Vec<DatasetColumnProfile>,
    pub rule_results: Vec<DatasetRuleResult>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DatasetProfileRecord {
    pub profile: serde_json::Value,
    pub score: f64,
    pub profiled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetQualityRule {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub name: String,
    pub rule_type: String,
    pub severity: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetQualityHistoryEntry {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub score: f64,
    pub passed_rules: i32,
    pub failed_rules: i32,
    pub alerts_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetQualityAlert {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub level: String,
    pub kind: String,
    pub message: String,
    pub status: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQualityRuleRequest {
    pub name: String,
    pub rule_type: String,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQualityRuleRequest {
    pub name: Option<String>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DatasetQualityResponse {
    pub profile: Option<DatasetQualityProfile>,
    pub score: Option<f64>,
    pub history: Vec<DatasetQualityHistoryEntry>,
    pub alerts: Vec<DatasetQualityAlert>,
    pub rules: Vec<DatasetQualityRule>,
    pub profiled_at: Option<DateTime<Utc>>,
}
