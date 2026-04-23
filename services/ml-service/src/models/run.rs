use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
	pub name: String,
	pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReference {
	pub id: Uuid,
	pub name: String,
	pub uri: String,
	pub artifact_type: String,
	pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRun {
	pub id: Uuid,
	pub experiment_id: Uuid,
	pub name: String,
	pub status: String,
	pub params: Value,
	pub metrics: Vec<MetricValue>,
	pub artifacts: Vec<ArtifactReference>,
	pub notes: String,
	pub source_dataset_ids: Vec<Uuid>,
	pub model_version_id: Option<Uuid>,
	pub started_at: Option<DateTime<Utc>>,
	pub finished_at: Option<DateTime<Utc>>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRunsResponse {
	pub data: Vec<ExperimentRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExperimentRunRequest {
	pub name: String,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default)]
	pub params: Value,
	#[serde(default)]
	pub metrics: Vec<MetricValue>,
	#[serde(default)]
	pub artifacts: Vec<ArtifactReference>,
	#[serde(default)]
	pub notes: Option<String>,
	#[serde(default)]
	pub source_dataset_ids: Vec<Uuid>,
	#[serde(default)]
	pub started_at: Option<DateTime<Utc>>,
	#[serde(default)]
	pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateExperimentRunRequest {
	pub status: Option<String>,
	pub params: Option<Value>,
	pub metrics: Option<Vec<MetricValue>>,
	pub artifacts: Option<Vec<ArtifactReference>>,
	pub notes: Option<String>,
	pub model_version_id: Option<Uuid>,
	pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareRunsRequest {
	pub run_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareRunsResponse {
	pub data: Vec<ExperimentRun>,
	pub metric_names: Vec<String>,
}
