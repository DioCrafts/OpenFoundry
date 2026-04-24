use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivorshipRule {
    pub field: String,
    pub strategy: String,
    pub source_priority: Vec<String>,
    pub fallback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeStrategy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub default_strategy: String,
    pub rules: Vec<SurvivorshipRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMergeStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub entity_type: Option<String>,
    pub default_strategy: Option<String>,
    pub rules: Vec<SurvivorshipRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMergeStrategyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub entity_type: Option<String>,
    pub default_strategy: Option<String>,
    pub rules: Option<Vec<SurvivorshipRule>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MergeStrategyRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub default_strategy: String,
    pub rules: SqlJson<Vec<SurvivorshipRule>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MergeStrategyRow> for MergeStrategy {
    fn from(value: MergeStrategyRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            entity_type: value.entity_type,
            default_strategy: value.default_strategy,
            rules: value.rules.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
