use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssetNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssetEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAssetLineageSummary {
    pub dataset_count: usize,
    pub run_count: usize,
    pub training_job_count: usize,
    pub model_count: usize,
    pub version_count: usize,
    pub deployment_count: usize,
    pub frameworks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentAssetLineageResponse {
    pub experiment_id: Uuid,
    pub objective_status: String,
    pub nodes: Vec<ModelAssetNode>,
    pub edges: Vec<ModelAssetEdge>,
    pub summary: ModelAssetLineageSummary,
}
