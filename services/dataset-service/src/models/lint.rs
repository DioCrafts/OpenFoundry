use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct DatasetLintSummary {
    pub resource_posture: String,
    pub total_findings: usize,
    pub high_severity: usize,
    pub medium_severity: usize,
    pub low_severity: usize,
    pub tracked_versions: usize,
    pub branch_count: usize,
    pub stale_branch_count: usize,
    pub materialized_view_count: usize,
    pub auto_refresh_view_count: usize,
    pub transaction_count: usize,
    pub failed_transaction_count: usize,
    pub pending_transaction_count: usize,
    pub enabled_rule_count: usize,
    pub active_alert_count: usize,
    pub object_count: usize,
    pub small_file_count: usize,
    pub largest_object_bytes: i64,
    pub average_object_size_bytes: i64,
    pub quality_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetLintFinding {
    pub code: String,
    pub title: String,
    pub severity: String,
    pub category: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub impact: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetLintRecommendation {
    pub code: String,
    pub priority: String,
    pub title: String,
    pub rationale: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetLintResponse {
    pub dataset_id: Uuid,
    pub dataset_name: String,
    pub analyzed_at: DateTime<Utc>,
    pub summary: DatasetLintSummary,
    pub findings: Vec<DatasetLintFinding>,
    pub recommendations: Vec<DatasetLintRecommendation>,
}
