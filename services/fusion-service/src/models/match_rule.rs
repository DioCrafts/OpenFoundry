use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockingStrategyConfig {
    pub strategy_type: String,
    pub key_fields: Vec<String>,
    pub window_size: i32,
    pub bucket_count: i32,
}

impl Default for BlockingStrategyConfig {
    fn default() -> Self {
        Self {
            strategy_type: "key-based".to_string(),
            key_fields: vec![
                "email".to_string(),
                "phone".to_string(),
                "display_name".to_string(),
            ],
            window_size: 5,
            bucket_count: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCondition {
    pub field: String,
    pub comparator: String,
    pub weight: f32,
    pub threshold: f32,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRule {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub blocking_strategy: BlockingStrategyConfig,
    pub conditions: Vec<MatchCondition>,
    pub review_threshold: f32,
    pub auto_merge_threshold: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMatchRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub entity_type: Option<String>,
    pub blocking_strategy: Option<BlockingStrategyConfig>,
    pub conditions: Vec<MatchCondition>,
    pub review_threshold: Option<f32>,
    pub auto_merge_threshold: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMatchRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub entity_type: Option<String>,
    pub blocking_strategy: Option<BlockingStrategyConfig>,
    pub conditions: Option<Vec<MatchCondition>>,
    pub review_threshold: Option<f32>,
    pub auto_merge_threshold: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MatchRuleRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub blocking_strategy: SqlJson<BlockingStrategyConfig>,
    pub conditions: SqlJson<Vec<MatchCondition>>,
    pub review_threshold: f32,
    pub auto_merge_threshold: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MatchRuleRow> for MatchRule {
    fn from(value: MatchRuleRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            entity_type: value.entity_type,
            blocking_strategy: value.blocking_strategy.0,
            conditions: value.conditions.0,
            review_threshold: value.review_threshold,
            auto_merge_threshold: value.auto_merge_threshold,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
