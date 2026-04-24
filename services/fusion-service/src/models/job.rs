use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

use super::match_rule::BlockingStrategyConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionJobConfig {
    pub source_labels: Vec<String>,
    pub record_count: i32,
    pub blocking_strategy_override: Option<BlockingStrategyConfig>,
    pub review_sampling_rate: f32,
}

impl Default for ResolutionJobConfig {
    fn default() -> Self {
        Self {
            source_labels: vec!["crm".to_string(), "erp".to_string(), "support".to_string()],
            record_count: 12,
            blocking_strategy_override: None,
            review_sampling_rate: 0.25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionJobMetrics {
    pub candidate_pairs: i32,
    pub matched_pairs: i32,
    pub review_pairs: i32,
    pub cluster_count: i32,
    pub golden_record_count: i32,
    pub precision_estimate: f32,
    pub recall_estimate: f32,
}

impl Default for FusionJobMetrics {
    fn default() -> Self {
        Self {
            candidate_pairs: 0,
            matched_pairs: 0,
            review_pairs: 0,
            cluster_count: 0,
            golden_record_count: 0,
            precision_estimate: 0.0,
            recall_estimate: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionJob {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub match_rule_id: Uuid,
    pub merge_strategy_id: Uuid,
    pub config: ResolutionJobConfig,
    pub metrics: FusionJobMetrics,
    pub last_run_summary: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateFusionJobRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub entity_type: Option<String>,
    pub match_rule_id: Uuid,
    pub merge_strategy_id: Uuid,
    pub config: Option<ResolutionJobConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResolutionJobResponse {
    pub job: FusionJob,
    pub cluster_ids: Vec<Uuid>,
    pub golden_record_ids: Vec<Uuid>,
    pub review_queue_item_ids: Vec<Uuid>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FusionJobRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub entity_type: String,
    pub match_rule_id: Uuid,
    pub merge_strategy_id: Uuid,
    pub config: SqlJson<ResolutionJobConfig>,
    pub metrics: SqlJson<FusionJobMetrics>,
    pub last_run_summary: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<FusionJobRow> for FusionJob {
    fn from(value: FusionJobRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            entity_type: value.entity_type,
            match_rule_id: value.match_rule_id,
            merge_strategy_id: value.merge_strategy_id,
            config: value.config.0,
            metrics: value.metrics.0,
            last_run_summary: value.last_run_summary,
            last_run_at: value.last_run_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
