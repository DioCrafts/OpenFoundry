pub mod deployment;
pub mod experiment;
pub mod feature;
pub mod model;
pub mod model_version;
pub mod prediction;
pub mod run;
pub mod training_job;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlStudioOverview {
    pub experiment_count: i64,
    pub active_run_count: i64,
    pub model_count: i64,
    pub production_model_count: i64,
    pub feature_count: i64,
    pub online_feature_count: i64,
    pub deployment_count: i64,
    pub ab_test_count: i64,
    pub drift_alert_count: i64,
    pub queued_training_jobs: i64,
}