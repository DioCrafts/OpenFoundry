use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::run::MetricValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingTrial {
    pub id: String,
    pub status: String,
    pub hyperparameters: Value,
    pub objective_metric: MetricValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: Uuid,
    pub experiment_id: Option<Uuid>,
    pub model_id: Option<Uuid>,
    pub name: String,
    pub status: String,
    pub dataset_ids: Vec<Uuid>,
    pub training_config: Value,
    pub hyperparameter_search: Value,
    pub objective_metric_name: String,
    pub trials: Vec<TrainingTrial>,
    pub best_model_version_id: Option<Uuid>,
    pub submitted_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTrainingJobsResponse {
    pub data: Vec<TrainingJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTrainingJobRequest {
    pub experiment_id: Option<Uuid>,
    pub model_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub dataset_ids: Vec<Uuid>,
    #[serde(default)]
    pub training_config: Value,
    pub hyperparameter_search: Option<Value>,
    pub objective_metric_name: Option<String>,
    #[serde(default)]
    pub auto_register_model_version: bool,
}
