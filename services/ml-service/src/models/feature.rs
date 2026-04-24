use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSample {
    pub entity_key: String,
    pub value: Value,
    pub observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDefinition {
    pub id: Uuid,
    pub name: String,
    pub entity_name: String,
    pub data_type: String,
    pub description: String,
    pub status: String,
    pub offline_source: String,
    pub transformation: String,
    pub online_enabled: bool,
    pub online_namespace: String,
    pub batch_schedule: String,
    pub freshness_sla_minutes: i32,
    pub tags: Vec<String>,
    pub samples: Vec<FeatureSample>,
    pub last_materialized_at: Option<DateTime<Utc>>,
    pub last_online_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFeaturesResponse {
    pub data: Vec<FeatureDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFeatureRequest {
    pub name: String,
    pub entity_name: String,
    pub data_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub offline_source: String,
    #[serde(default)]
    pub transformation: String,
    #[serde(default)]
    pub online_enabled: bool,
    #[serde(default)]
    pub online_namespace: String,
    #[serde(default = "default_batch_schedule")]
    pub batch_schedule: String,
    #[serde(default = "default_freshness_sla")]
    pub freshness_sla_minutes: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub samples: Vec<FeatureSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateFeatureRequest {
    pub name: Option<String>,
    pub entity_name: Option<String>,
    pub data_type: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub offline_source: Option<String>,
    pub transformation: Option<String>,
    pub online_enabled: Option<bool>,
    pub online_namespace: Option<String>,
    pub batch_schedule: Option<String>,
    pub freshness_sla_minutes: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterializeFeatureRequest {
    #[serde(default)]
    pub samples: Vec<FeatureSample>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineFeatureSnapshot {
    pub feature_id: Uuid,
    pub namespace: String,
    pub source: String,
    pub values: Vec<FeatureSample>,
    pub fetched_at: DateTime<Utc>,
}

fn default_batch_schedule() -> String {
    "0 * * * *".to_string()
}

fn default_freshness_sla() -> i32 {
    60
}
