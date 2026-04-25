use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::models::{
    sink::{ConnectorCatalogEntry, LiveTailEvent},
    stream::StreamDefinition,
};

pub fn catalog_entry(stream: &StreamDefinition) -> ConnectorCatalogEntry {
    ConnectorCatalogEntry {
        connector_type: "nats".to_string(),
        direction: "source".to_string(),
        endpoint: stream.source_binding.endpoint.clone(),
        status: "healthy".to_string(),
        backlog: 9,
        throughput_per_second: 236.0,
        details: json!({
            "format": stream.source_binding.format,
            "subject": stream.source_binding.endpoint
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
            connector_type: "nats".to_string(),
            payload: json!({
                "payment_id": "pay-9001",
                "customer_id": "cust-22",
                "status": "authorized",
                "risk_band": "medium"
            }),
            event_time: now - Duration::seconds(11),
            processing_time: now - Duration::seconds(10),
            tags: vec![
                "pattern:payment-then-order".to_string(),
                "source:nats".to_string(),
            ],
        },
        LiveTailEvent {
            id: format!("{}-evt-2", stream.name.to_lowercase().replace(' ', "-")),
            topology_id,
            stream_name: stream.name.clone(),
            connector_type: "nats".to_string(),
            payload: json!({
                "payment_id": "pay-9008",
                "customer_id": "cust-31",
                "status": "captured",
                "risk_band": "low"
            }),
            event_time: now - Duration::seconds(4),
            processing_time: now - Duration::seconds(3),
            tags: vec![
                "join-key:customer_id".to_string(),
                "source:nats".to_string(),
            ],
        },
    ]
}
