use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::models::{
    sink::{ConnectorCatalogEntry, LiveTailEvent},
    stream::StreamDefinition,
};

pub fn catalog_entry(stream: &StreamDefinition) -> ConnectorCatalogEntry {
    ConnectorCatalogEntry {
        connector_type: "kafka".to_string(),
        direction: "source".to_string(),
        endpoint: stream.source_binding.endpoint.clone(),
        status: "healthy".to_string(),
        backlog: 18,
        throughput_per_second: 482.0,
        details: json!({
            "format": stream.source_binding.format,
            "consumer_group": stream.name.to_lowercase().replace(' ', "-")
        }),
    }
}

#[allow(dead_code)]
pub fn sample_events(stream: &StreamDefinition, topology_id: Uuid) -> Vec<LiveTailEvent> {
    let now = Utc::now();
    vec![
        LiveTailEvent {
            id: format!("{}-evt-1", stream.name.to_lowercase().replace(' ', "-")),
            topology_id,
            stream_name: stream.name.clone(),
            connector_type: "kafka".to_string(),
            payload: json!({
                "order_id": "ord-1842",
                "customer_id": "cust-22",
                "amount": 1420.5,
                "currency": "USD"
            }),
            event_time: now - Duration::seconds(14),
            processing_time: now - Duration::seconds(13),
            tags: vec![
                "join-key:customer_id".to_string(),
                "source:kafka".to_string(),
            ],
        },
        LiveTailEvent {
            id: format!("{}-evt-2", stream.name.to_lowercase().replace(' ', "-")),
            topology_id,
            stream_name: stream.name.clone(),
            connector_type: "kafka".to_string(),
            payload: json!({
                "order_id": "ord-1849",
                "customer_id": "cust-22",
                "amount": 310.0,
                "currency": "USD"
            }),
            event_time: now - Duration::seconds(7),
            processing_time: now - Duration::seconds(6),
            tags: vec!["window:5m".to_string(), "source:kafka".to_string()],
        },
    ]
}
