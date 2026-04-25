use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::models::{
    sink::{ConnectorCatalogEntry, LiveTailEvent},
    stream::StreamDefinition,
};

pub fn catalog_entry(stream: &StreamDefinition) -> ConnectorCatalogEntry {
    ConnectorCatalogEntry {
        connector_type: "http".to_string(),
        direction: "source".to_string(),
        endpoint: stream.source_binding.endpoint.clone(),
        status: "healthy".to_string(),
        backlog: 4,
        throughput_per_second: 92.0,
        details: json!({
            "method": "POST",
            "format": stream.source_binding.format
        }),
    }
}

#[allow(dead_code)]
pub fn sample_events(stream: &StreamDefinition, topology_id: Uuid) -> Vec<LiveTailEvent> {
    let now = Utc::now();
    vec![LiveTailEvent {
        id: format!("{}-evt-1", stream.name.to_lowercase().replace(' ', "-")),
        topology_id,
        stream_name: stream.name.clone(),
        connector_type: "http".to_string(),
        payload: json!({
            "tenant": "acme",
            "event_type": "cart_checkout",
            "customer_id": "cust-51",
            "value": 88.2
        }),
        event_time: now - Duration::seconds(5),
        processing_time: now - Duration::seconds(4),
        tags: vec!["source:http".to_string(), "window:custom".to_string()],
    }]
}
