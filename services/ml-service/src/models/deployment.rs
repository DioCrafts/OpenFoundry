use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplitEntry {
    pub model_version_id: Uuid,
    pub label: String,
    pub allocation: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMetric {
    pub name: String,
    pub score: f64,
    pub threshold: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub generated_at: DateTime<Utc>,
    pub dataset_metrics: Vec<DriftMetric>,
    pub concept_metrics: Vec<DriftMetric>,
    pub recommend_retraining: bool,
    pub auto_retraining_job_id: Option<Uuid>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDeployment {
    pub id: Uuid,
    pub model_id: Uuid,
    pub name: String,
    pub status: String,
    pub strategy_type: String,
    pub endpoint_path: String,
    pub traffic_split: Vec<TrafficSplitEntry>,
    pub monitoring_window: String,
    pub baseline_dataset_id: Option<Uuid>,
    pub drift_report: Option<DriftReport>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDeploymentsResponse {
    pub data: Vec<ModelDeployment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeploymentRequest {
    pub model_id: Uuid,
    pub name: String,
    #[serde(default = "default_strategy_type")]
    pub strategy_type: String,
    pub endpoint_path: String,
    #[serde(default)]
    pub traffic_split: Vec<TrafficSplitEntry>,
    #[serde(default = "default_monitoring_window")]
    pub monitoring_window: String,
    pub baseline_dataset_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateDeploymentRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub strategy_type: Option<String>,
    pub endpoint_path: Option<String>,
    pub traffic_split: Option<Vec<TrafficSplitEntry>>,
    pub monitoring_window: Option<String>,
    pub baseline_dataset_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateDriftReportRequest {
    pub baseline_rows: Option<i64>,
    pub observed_rows: Option<i64>,
    #[serde(default)]
    pub auto_retrain: bool,
}

fn default_strategy_type() -> String {
    "single".to_string()
}

fn default_monitoring_window() -> String {
    "24h".to_string()
}
