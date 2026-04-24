use serde_json::json;

use crate::models::{sink::ConnectorCatalogEntry, stream::ConnectorBinding};

pub fn catalog_entry(binding: &ConnectorBinding) -> ConnectorCatalogEntry {
    ConnectorCatalogEntry {
        connector_type: "websocket".to_string(),
        direction: "sink".to_string(),
        endpoint: binding.endpoint.clone(),
        status: "ready".to_string(),
        backlog: 1,
        throughput_per_second: 240.0,
        details: json!({
            "format": binding.format,
            "fanout_channels": 2
        }),
    }
}
