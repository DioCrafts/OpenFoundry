use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCatalogEntry {
    pub connector_type: String,
    pub direction: String,
    pub endpoint: String,
    pub status: String,
    pub backlog: i32,
    pub throughput_per_second: f32,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureSnapshot {
    pub queue_depth: i32,
    pub queue_capacity: i32,
    pub lag_ms: i32,
    pub throttle_factor: f32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStoreSnapshot {
    pub backend: String,
    pub namespace: String,
    pub key_count: i32,
    pub disk_usage_mb: i32,
    pub checkpoint_count: i32,
    pub last_checkpoint_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAggregate {
    pub window_name: String,
    pub window_type: String,
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub group_key: String,
    pub measure_name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTailEvent {
    pub id: String,
    pub topology_id: Uuid,
    pub stream_name: String,
    pub connector_type: String,
    pub payload: Value,
    pub event_time: DateTime<Utc>,
    pub processing_time: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CepMatch {
    pub pattern_name: String,
    pub matched_sequence: Vec<String>,
    pub confidence: f32,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveTailResponse {
    pub events: Vec<LiveTailEvent>,
    pub matches: Vec<CepMatch>,
}
