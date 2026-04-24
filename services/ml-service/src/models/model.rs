use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredModel {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub problem_type: String,
    pub status: String,
    pub tags: Vec<String>,
    pub owner_id: Option<Uuid>,
    pub current_stage: String,
    pub latest_version_number: Option<i32>,
    pub active_deployment_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub data: Vec<RegisteredModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_problem_type")]
    pub problem_type: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub problem_type: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
}

fn default_problem_type() -> String {
    "classification".to_string()
}
