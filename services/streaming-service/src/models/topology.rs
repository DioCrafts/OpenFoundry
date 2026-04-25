use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

use super::{
    sink::{
        BackpressureSnapshot, CepMatch, ConnectorCatalogEntry, LiveTailEvent, StateStoreSnapshot,
        WindowAggregate,
    },
    stream::ConnectorBinding,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub stream_id: Option<Uuid>,
    pub window_id: Option<Uuid>,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinDefinition {
    pub join_type: String,
    pub left_stream_id: Uuid,
    pub right_stream_id: Uuid,
    pub table_name: String,
    pub key_fields: Vec<String>,
    pub window_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CepDefinition {
    pub pattern_name: String,
    pub sequence: Vec<String>,
    pub within_seconds: i32,
    pub output_stream: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressurePolicy {
    pub max_in_flight: i32,
    pub queue_capacity: i32,
    pub throttle_strategy: String,
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        Self {
            max_in_flight: 512,
            queue_capacity: 2048,
            throttle_strategy: "credit-based".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub join_definition: Option<JoinDefinition>,
    pub cep_definition: Option<CepDefinition>,
    pub backpressure_policy: BackpressurePolicy,
    pub source_stream_ids: Vec<Uuid>,
    pub sink_bindings: Vec<ConnectorBinding>,
    pub state_backend: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTopologyRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub join_definition: Option<JoinDefinition>,
    pub cep_definition: Option<CepDefinition>,
    pub backpressure_policy: Option<BackpressurePolicy>,
    pub source_stream_ids: Vec<Uuid>,
    pub sink_bindings: Vec<ConnectorBinding>,
    pub state_backend: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTopologyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub nodes: Option<Vec<TopologyNode>>,
    pub edges: Option<Vec<TopologyEdge>>,
    pub join_definition: Option<JoinDefinition>,
    pub cep_definition: Option<CepDefinition>,
    pub backpressure_policy: Option<BackpressurePolicy>,
    pub source_stream_ids: Option<Vec<Uuid>>,
    pub sink_bindings: Option<Vec<ConnectorBinding>>,
    pub state_backend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyRunMetrics {
    pub input_events: i32,
    pub output_events: i32,
    pub avg_latency_ms: i32,
    pub p95_latency_ms: i32,
    pub throughput_per_second: f32,
    pub dropped_events: i32,
    pub backpressure_ratio: f32,
    pub join_output_rows: i32,
    pub cep_match_count: i32,
    pub state_entries: i32,
}

impl Default for TopologyRunMetrics {
    fn default() -> Self {
        Self {
            input_events: 0,
            output_events: 0,
            avg_latency_ms: 0,
            p95_latency_ms: 0,
            throughput_per_second: 0.0,
            dropped_events: 0,
            backpressure_ratio: 0.0,
            join_output_rows: 0,
            cep_match_count: 0,
            state_entries: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyRun {
    pub id: Uuid,
    pub topology_id: Uuid,
    pub status: String,
    pub metrics: TopologyRunMetrics,
    pub aggregate_windows: Vec<WindowAggregate>,
    pub live_tail: Vec<LiveTailEvent>,
    pub cep_matches: Vec<CepMatch>,
    pub state_snapshot: StateStoreSnapshot,
    pub backpressure_snapshot: BackpressureSnapshot,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyRuntimePreview {
    pub metrics: TopologyRunMetrics,
    pub aggregate_windows: Vec<WindowAggregate>,
    pub backpressure_snapshot: BackpressureSnapshot,
    pub state_snapshot: StateStoreSnapshot,
    pub backlog_events: i32,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopologyRuntimeSnapshot {
    pub topology: TopologyDefinition,
    pub latest_run: Option<TopologyRun>,
    pub preview: Option<TopologyRuntimePreview>,
    pub connector_statuses: Vec<ConnectorCatalogEntry>,
    pub latest_events: Vec<LiveTailEvent>,
    pub latest_matches: Vec<CepMatch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayTopologyRequest {
    pub stream_ids: Option<Vec<Uuid>>,
    pub from_sequence_no: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayTopologyResponse {
    pub topology_id: Uuid,
    pub stream_ids: Vec<Uuid>,
    pub replay_from_sequence_no: Option<i64>,
    pub restored_event_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct TopologyRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub nodes: SqlJson<Vec<TopologyNode>>,
    pub edges: SqlJson<Vec<TopologyEdge>>,
    pub join_definition: Option<SqlJson<JoinDefinition>>,
    pub cep_definition: Option<SqlJson<CepDefinition>>,
    pub backpressure_policy: SqlJson<BackpressurePolicy>,
    pub source_stream_ids: SqlJson<Vec<Uuid>>,
    pub sink_bindings: SqlJson<Vec<ConnectorBinding>>,
    pub state_backend: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TopologyRunRow {
    pub id: Uuid,
    pub topology_id: Uuid,
    pub status: String,
    pub metrics: SqlJson<TopologyRunMetrics>,
    pub aggregate_windows: SqlJson<Vec<WindowAggregate>>,
    pub live_tail: SqlJson<Vec<LiveTailEvent>>,
    pub cep_matches: SqlJson<Vec<CepMatch>>,
    pub state_snapshot: SqlJson<StateStoreSnapshot>,
    pub backpressure_snapshot: SqlJson<BackpressureSnapshot>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TopologyRow> for TopologyDefinition {
    fn from(value: TopologyRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            nodes: value.nodes.0,
            edges: value.edges.0,
            join_definition: value.join_definition.map(|item| item.0),
            cep_definition: value.cep_definition.map(|item| item.0),
            backpressure_policy: value.backpressure_policy.0,
            source_stream_ids: value.source_stream_ids.0,
            sink_bindings: value.sink_bindings.0,
            state_backend: value.state_backend,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TopologyRunRow> for TopologyRun {
    fn from(value: TopologyRunRow) -> Self {
        Self {
            id: value.id,
            topology_id: value.topology_id,
            status: value.status,
            metrics: value.metrics.0,
            aggregate_windows: value.aggregate_windows.0,
            live_tail: value.live_tail.0,
            cep_matches: value.cep_matches.0,
            state_snapshot: value.state_snapshot.0,
            backpressure_snapshot: value.backpressure_snapshot.0,
            started_at: value.started_at,
            completed_at: value.completed_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
