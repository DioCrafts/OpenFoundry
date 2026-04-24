use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureContribution {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionOutput {
    pub record_id: String,
    pub variant: String,
    pub model_version_id: Uuid,
    pub predicted_label: String,
    pub score: f64,
    pub confidence: f64,
    pub contributions: Vec<FeatureContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimePredictionRequest {
    #[serde(default)]
    pub inputs: Vec<Value>,
    #[serde(default)]
    pub explain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimePredictionResponse {
    pub deployment_id: Uuid,
    pub outputs: Vec<PredictionOutput>,
    pub predicted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBatchPredictionRequest {
    pub deployment_id: Uuid,
    #[serde(default)]
    pub records: Vec<Value>,
    pub output_destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPredictionJob {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub status: String,
    pub record_count: i64,
    pub output_destination: Option<String>,
    pub outputs: Vec<PredictionOutput>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBatchPredictionsResponse {
    pub data: Vec<BatchPredictionJob>,
}
