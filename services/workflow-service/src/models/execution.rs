use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkflowRun {
	pub id: Uuid,
	pub workflow_id: Uuid,
	pub trigger_type: String,
	pub status: String,
	pub started_by: Option<Uuid>,
	pub current_step_id: Option<String>,
	pub context: Value,
	pub error_message: Option<String>,
	pub started_at: DateTime<Utc>,
	pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct StartRunRequest {
	#[serde(default)]
	pub context: Value,
}

#[derive(Debug, Deserialize)]
pub struct TriggerEventRequest {
	#[serde(default)]
	pub context: Value,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
	pub page: Option<i64>,
	pub per_page: Option<i64>,
}
