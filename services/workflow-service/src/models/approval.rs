use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkflowApproval {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_run_id: Uuid,
    pub step_id: String,
    pub title: String,
    pub instructions: String,
    pub assigned_to: Option<Uuid>,
    pub status: String,
    pub decision: Option<String>,
    pub payload: Value,
    pub requested_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListApprovalsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalDecisionRequest {
    pub decision: String,
    pub comment: Option<String>,
    #[serde(default)]
    pub payload: Value,
}
