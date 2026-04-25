use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

fn default_skip_unchanged() -> bool {
    true
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub trigger_type: String,
    pub started_by: Option<Uuid>,
    pub attempt_number: i32,
    pub started_from_node_id: Option<String>,
    pub retry_of_run_id: Option<Uuid>,
    pub execution_context: Value,
    pub node_results: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TriggerPipelineRequest {
    pub from_node_id: Option<String>,
    pub context: Option<Value>,
    #[serde(default = "default_skip_unchanged")]
    pub skip_unchanged: bool,
}

#[derive(Debug, Deserialize)]
pub struct RetryPipelineRunRequest {
    pub from_node_id: Option<String>,
    #[serde(default = "default_skip_unchanged")]
    pub skip_unchanged: bool,
}
