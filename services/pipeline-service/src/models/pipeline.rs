use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: Uuid,
    pub dag: serde_json::Value,
    pub status: String,
    pub schedule_config: Value,
    pub retry_policy: Value,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Pipeline {
    pub fn parsed_nodes(&self) -> Result<Vec<PipelineNode>, String> {
        serde_json::from_value(self.dag.clone()).map_err(|error| error.to_string())
    }

    pub fn schedule(&self) -> PipelineScheduleConfig {
        serde_json::from_value(self.schedule_config.clone()).unwrap_or_default()
    }

    pub fn parsed_retry_policy(&self) -> PipelineRetryPolicy {
        serde_json::from_value(self.retry_policy.clone()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PipelineScheduleConfig {
    pub enabled: bool,
    pub cron: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineRetryPolicy {
    pub max_attempts: u32,
    pub retry_on_failure: bool,
    pub allow_partial_reexecution: bool,
}

impl Default for PipelineRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            retry_on_failure: false,
            allow_partial_reexecution: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineColumnMapping {
    pub source_dataset_id: Option<Uuid>,
    pub source_column: String,
    pub target_column: String,
}

/// A single node in the pipeline DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    pub id: String,
    pub label: String,
    pub transform_type: String, // "sql", "python", "passthrough"
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub input_dataset_ids: Vec<Uuid>,
    #[serde(default)]
    pub output_dataset_id: Option<Uuid>,
}

impl PipelineNode {
    pub fn column_mappings(&self) -> Vec<PipelineColumnMapping> {
        self.config
            .get("column_mappings")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub nodes: Vec<PipelineNode>,
    pub schedule_config: Option<PipelineScheduleConfig>,
    pub retry_policy: Option<PipelineRetryPolicy>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePipelineRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub nodes: Option<Vec<PipelineNode>>,
    pub schedule_config: Option<PipelineScheduleConfig>,
    pub retry_policy: Option<PipelineRetryPolicy>,
}

#[derive(Debug, Deserialize)]
pub struct ListPipelinesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListPipelinesResponse {
    pub data: Vec<Pipeline>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
