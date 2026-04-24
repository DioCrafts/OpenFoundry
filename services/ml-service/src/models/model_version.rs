use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::run::MetricValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    pub id: Uuid,
    pub model_id: Uuid,
    pub version_number: i32,
    pub version_label: String,
    pub stage: String,
    pub source_run_id: Option<Uuid>,
    pub training_job_id: Option<Uuid>,
    pub hyperparameters: Value,
    pub metrics: Vec<MetricValue>,
    pub artifact_uri: Option<String>,
    pub schema: Value,
    pub created_at: DateTime<Utc>,
    pub promoted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelVersionsResponse {
    pub data: Vec<ModelVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateModelVersionRequest {
    pub version_label: Option<String>,
    pub stage: Option<String>,
    pub source_run_id: Option<Uuid>,
    pub training_job_id: Option<Uuid>,
    pub hyperparameters: Option<Value>,
    pub metrics: Option<Vec<MetricValue>>,
    pub artifact_uri: Option<String>,
    pub schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionModelVersionRequest {
    pub stage: String,
}
