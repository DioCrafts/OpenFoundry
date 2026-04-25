pub mod dead_letter;
pub mod sink;
pub mod stream;
pub mod topology;
pub mod window;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ListResponse<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamingOverview {
    pub stream_count: i64,
    pub active_topology_count: i64,
    pub window_count: i64,
    pub connector_count: i64,
    pub running_topology_count: i64,
    pub backpressured_topology_count: i64,
    pub live_event_count: i64,
}
