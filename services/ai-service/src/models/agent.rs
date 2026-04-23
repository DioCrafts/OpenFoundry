use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{types::Json, FromRow};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMemorySnapshot {
	#[serde(default)]
	pub short_term_notes: Vec<String>,
	#[serde(default)]
	pub long_term_references: Vec<String>,
	pub last_run_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanStep {
	pub id: String,
	pub title: String,
	pub description: String,
	pub tool_name: Option<String>,
	pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionTrace {
	pub step_id: String,
	pub title: String,
	pub tool_name: Option<String>,
	pub observation: String,
	pub output: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
	pub id: Uuid,
	pub name: String,
	pub description: String,
	pub status: String,
	pub system_prompt: String,
	pub objective: String,
	pub tool_ids: Vec<Uuid>,
	pub planning_strategy: String,
	pub max_iterations: i32,
	pub memory: AgentMemorySnapshot,
	pub last_execution_at: Option<DateTime<Utc>>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsResponse {
	pub data: Vec<AgentDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
	pub name: String,
	#[serde(default)]
	pub description: String,
	#[serde(default = "default_agent_status")]
	pub status: String,
	#[serde(default)]
	pub system_prompt: String,
	#[serde(default)]
	pub objective: String,
	#[serde(default)]
	pub tool_ids: Vec<Uuid>,
	#[serde(default = "default_planning_strategy")]
	pub planning_strategy: String,
	#[serde(default = "default_max_iterations")]
	pub max_iterations: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateAgentRequest {
	pub name: Option<String>,
	pub description: Option<String>,
	pub status: Option<String>,
	pub system_prompt: Option<String>,
	pub objective: Option<String>,
	pub tool_ids: Option<Vec<Uuid>>,
	pub planning_strategy: Option<String>,
	pub max_iterations: Option<i32>,
	pub memory: Option<AgentMemorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAgentRequest {
	pub user_message: String,
	pub objective: Option<String>,
	pub knowledge_base_id: Option<Uuid>,
	#[serde(default = "default_json_object")]
	pub context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResponse {
	pub agent_id: Uuid,
	pub steps: Vec<AgentPlanStep>,
	pub traces: Vec<AgentExecutionTrace>,
	pub final_response: String,
	pub used_tool_names: Vec<String>,
	pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct AgentRow {
	pub id: Uuid,
	pub name: String,
	pub description: String,
	pub status: String,
	pub system_prompt: String,
	pub objective: String,
	pub tool_ids: Json<Vec<Uuid>>,
	pub planning_strategy: String,
	pub max_iterations: i32,
	pub memory: Json<AgentMemorySnapshot>,
	pub last_execution_at: Option<DateTime<Utc>>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<AgentRow> for AgentDefinition {
	fn from(value: AgentRow) -> Self {
		Self {
			id: value.id,
			name: value.name,
			description: value.description,
			status: value.status,
			system_prompt: value.system_prompt,
			objective: value.objective,
			tool_ids: value.tool_ids.0,
			planning_strategy: value.planning_strategy,
			max_iterations: value.max_iterations,
			memory: value.memory.0,
			last_execution_at: value.last_execution_at,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

fn default_agent_status() -> String {
	"active".to_string()
}

fn default_planning_strategy() -> String {
	"plan-act-observe".to_string()
}

fn default_max_iterations() -> i32 {
	3
}

fn default_json_object() -> Value {
	json!({})
}
