use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConnectorAgent {
    pub id: Uuid,
    pub name: String,
    pub agent_url: String,
    pub owner_id: Uuid,
    pub status: String,
    pub capabilities: serde_json::Value,
    pub metadata: serde_json::Value,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    pub agent_url: String,
    #[serde(default)]
    pub capabilities: serde_json::Value,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AgentHeartbeatRequest {
    #[serde(default)]
    pub capabilities: serde_json::Value,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
