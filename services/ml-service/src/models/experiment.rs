use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::run::MetricValue;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelingObjectiveSpec {
    #[serde(default = "default_objective_status")]
    pub status: String,
    #[serde(default)]
    pub deployment_target: String,
    #[serde(default)]
    pub stakeholders: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub linked_dataset_ids: Vec<Uuid>,
    #[serde(default)]
    pub linked_model_ids: Vec<Uuid>,
    #[serde(default)]
    pub documentation_uri: String,
    #[serde(default)]
    pub collaboration_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub objective: String,
    pub objective_spec: ModelingObjectiveSpec,
    pub task_type: String,
    pub primary_metric: String,
    pub status: String,
    pub tags: Vec<String>,
    pub run_count: i64,
    pub best_metric: Option<MetricValue>,
    pub owner_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListExperimentsResponse {
    pub data: Vec<Experiment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExperimentRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default = "default_task_type")]
    pub task_type: String,
    #[serde(default = "default_primary_metric")]
    pub primary_metric: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objective_spec: ModelingObjectiveSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateExperimentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub task_type: Option<String>,
    pub primary_metric: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub objective_spec: Option<ModelingObjectiveSpec>,
}

fn default_task_type() -> String {
    "classification".to_string()
}

fn default_primary_metric() -> String {
    "accuracy".to_string()
}

fn default_objective_status() -> String {
    "draft".to_string()
}
