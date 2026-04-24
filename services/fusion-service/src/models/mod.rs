pub mod cluster;
pub mod golden_record;
pub mod job;
pub mod match_rule;
pub mod merge_strategy;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FusionOverview {
    pub rule_count: i64,
    pub active_job_count: i64,
    pub completed_job_count: i64,
    pub cluster_count: i64,
    pub pending_review_count: i64,
    pub golden_record_count: i64,
    pub auto_merged_cluster_count: i64,
}
