pub mod dataset_sink;
pub mod http_source;
pub mod kafka_source;
pub mod nats_source;
pub mod websocket_sink;

use crate::models::{
    sink::{ConnectorCatalogEntry, LiveTailEvent},
    stream::{ConnectorBinding, StreamDefinition},
    topology::TopologyDefinition,
};

pub fn catalog_entries(
    topology: &TopologyDefinition,
    streams: &[StreamDefinition],
) -> Vec<ConnectorCatalogEntry> {
    let mut entries = Vec::new();

    for stream in streams
        .iter()
        .filter(|stream| topology.source_stream_ids.contains(&stream.id))
    {
        let entry = match stream.source_binding.connector_type.as_str() {
            "kafka" => kafka_source::catalog_entry(stream),
            "nats" => nats_source::catalog_entry(stream),
            "http" => http_source::catalog_entry(stream),
            _ => fallback_catalog_entry(&stream.source_binding, "source"),
        };
        entries.push(entry);
    }

    for sink in &topology.sink_bindings {
        let entry = match sink.connector_type.as_str() {
            "dataset" => dataset_sink::catalog_entry(sink),
            "websocket" => websocket_sink::catalog_entry(sink),
            _ => fallback_catalog_entry(sink, "sink"),
        };
        entries.push(entry);
    }

    entries
}

#[allow(dead_code)]
pub fn live_events(
    topology: &TopologyDefinition,
    streams: &[StreamDefinition],
) -> Vec<LiveTailEvent> {
    let mut events = Vec::new();

    for stream in streams
        .iter()
        .filter(|stream| topology.source_stream_ids.contains(&stream.id))
    {
        let mut source_events = match stream.source_binding.connector_type.as_str() {
            "kafka" => kafka_source::sample_events(stream, topology.id),
            "nats" => nats_source::sample_events(stream, topology.id),
            "http" => http_source::sample_events(stream, topology.id),
            _ => Vec::new(),
        };
        events.append(&mut source_events);
    }

    events.sort_by_key(|event| event.processing_time);
    events.reverse();
    events.truncate(16);
    events
}

fn fallback_catalog_entry(binding: &ConnectorBinding, direction: &str) -> ConnectorCatalogEntry {
    ConnectorCatalogEntry {
        connector_type: binding.connector_type.clone(),
        direction: direction.to_string(),
        endpoint: binding.endpoint.clone(),
        status: "healthy".to_string(),
        backlog: 12,
        throughput_per_second: 128.0,
        details: binding.config.clone(),
    }
}
