use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBranchCondition {
    pub field: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBranch {
    pub condition: WorkflowBranchCondition,
    pub next_step_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub next_step_id: Option<String>,
    #[serde(default)]
    pub branches: Vec<WorkflowBranch>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: Uuid,
    pub status: String,
    pub trigger_type: String,
    pub trigger_config: Value,
    pub steps: Value,
    pub webhook_secret: Option<String>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkflowDefinition {
    pub fn parsed_steps(&self) -> Result<Vec<WorkflowStep>, String> {
        serde_json::from_value(self.steps.clone()).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub trigger_type: String,
    #[serde(default)]
    pub trigger_config: Value,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<Value>,
    pub steps: Option<Vec<WorkflowStep>>,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkflowsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub trigger_type: Option<String>,
    pub status: Option<String>,
}
