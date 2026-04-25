use serde_json::{Value, json};

use super::{
    ConnectionTestResult, SyncPayload, add_source_signature, basic_discovered_source,
    virtual_table_response,
};
use crate::{
    AppState,
    models::registration::{DiscoveredSource, VirtualTableQueryRequest, VirtualTableQueryResponse},
};

#[derive(Debug, Clone)]
struct TopicEntry {
    selector: String,
    display_name: String,
    sample_messages: Vec<Value>,
    partitions: i64,
    metadata: Value,
}

pub fn validate_config(config: &Value) -> Result<(), String> {
    if string_field(config, "bootstrap_servers")
        .or_else(|| string_field(config, "brokers"))
        .is_none()
    {
        return Err("kafka connector requires 'bootstrap_servers' or 'brokers'".to_string());
    }

    let topics = parse_topics(config)?;
    if topics.is_empty() {
        return Err("kafka connector requires at least one topic in 'topics'".to_string());
    }

    Ok(())
}

pub async fn test_connection(
    _state: &AppState,
    config: &Value,
    _agent_url: Option<&str>,
) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;
    let topics = parse_topics(config)?;
    Ok(ConnectionTestResult {
        success: true,
        message: format!("validated kafka catalog with {} topic(s)", topics.len()),
        latency_ms: 0,
        details: Some(json!({
            "bootstrap_servers": string_field(config, "bootstrap_servers")
                .or_else(|| string_field(config, "brokers")),
            "topic_count": topics.len(),
            "mode": "catalog_backed",
        })),
    })
}

pub async fn discover_sources(
    _state: &AppState,
    config: &Value,
    _agent_url: Option<&str>,
) -> Result<Vec<DiscoveredSource>, String> {
    validate_config(config)?;
    Ok(parse_topics(config)?
        .into_iter()
        .map(|topic| {
            basic_discovered_source(
                topic.selector,
                topic.display_name,
                "kafka_topic",
                topic.metadata,
            )
        })
        .collect())
}

pub async fn query_virtual_table(
    _state: &AppState,
    config: &Value,
    request: &VirtualTableQueryRequest,
    _agent_url: Option<&str>,
) -> Result<VirtualTableQueryResponse, String> {
    validate_config(config)?;
    let topic = find_topic(config, &request.selector)?;
    let rows = topic
        .sample_messages
        .iter()
        .take(request.limit.unwrap_or(50).clamp(1, 500))
        .cloned()
        .collect::<Vec<_>>();

    Ok(virtual_table_response(
        &request.selector,
        rows,
        json!({
            "bootstrap_servers": string_field(config, "bootstrap_servers")
                .or_else(|| string_field(config, "brokers")),
            "partitions": topic.partitions,
            "entry": topic.metadata,
        }),
    ))
}

pub async fn fetch_dataset(
    _state: &AppState,
    config: &Value,
    selector: &str,
    _agent_url: Option<&str>,
) -> Result<SyncPayload, String> {
    validate_config(config)?;
    let topic = find_topic(config, selector)?;
    let rows_synced = topic.sample_messages.len() as i64;
    let mut payload = SyncPayload {
        bytes: serde_json::to_vec(&topic.sample_messages).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced,
        file_name: format!("{}.json", sanitize_file_stem(selector)),
        metadata: json!({
            "bootstrap_servers": string_field(config, "bootstrap_servers")
                .or_else(|| string_field(config, "brokers")),
            "topic": selector,
            "partitions": topic.partitions,
            "entry": topic.metadata,
        }),
    };
    add_source_signature(&mut payload);
    Ok(payload)
}

fn find_topic(config: &Value, selector: &str) -> Result<TopicEntry, String> {
    parse_topics(config)?
        .into_iter()
        .find(|topic| topic.selector == selector)
        .ok_or_else(|| format!("kafka topic '{selector}' is not configured"))
}

fn parse_topics(config: &Value) -> Result<Vec<TopicEntry>, String> {
    let topics = config
        .get("topics")
        .and_then(Value::as_array)
        .ok_or_else(|| "kafka connector requires 'topics' to be an array".to_string())?;

    let mut parsed = Vec::with_capacity(topics.len());
    for (index, topic) in topics.iter().enumerate() {
        if let Some(selector) = topic
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            parsed.push(TopicEntry {
                selector: selector.to_string(),
                display_name: selector.to_string(),
                sample_messages: Vec::new(),
                partitions: 1,
                metadata: json!({ "topic": selector }),
            });
            continue;
        }

        let selector = topic
            .get("selector")
            .or_else(|| topic.get("topic"))
            .or_else(|| topic.get("name"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                format!("kafka connector topics[{index}] requires 'selector', 'topic' or 'name'")
            })?
            .to_string();
        let display_name = topic
            .get("display_name")
            .or_else(|| topic.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(&selector)
            .to_string();
        let sample_messages = topic
            .get("sample_messages")
            .or_else(|| topic.get("preview_rows"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let partitions = topic
            .get("partitions")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            .max(1);
        let mut metadata = topic.clone();
        if let Some(object) = metadata.as_object_mut() {
            object.remove("sample_messages");
            object.remove("preview_rows");
        }

        parsed.push(TopicEntry {
            selector,
            display_name,
            sample_messages,
            partitions,
            metadata,
        });
    }

    Ok(parsed)
}

fn string_field<'a>(config: &'a Value, field: &str) -> Option<&'a str> {
    config
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn sanitize_file_stem(selector: &str) -> String {
    let stem = selector
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "kafka_sync".to_string()
    } else {
        stem.chars().take(64).collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{find_topic, parse_topics, validate_config};

    #[test]
    fn parses_string_and_object_topics() {
        let topics = parse_topics(&json!({
            "topics": [
                "orders",
                {
                    "selector": "payments",
                    "display_name": "Payments",
                    "partitions": 6,
                    "sample_messages": [{ "payment_id": "pay-1" }]
                }
            ]
        }))
        .expect("topics should parse");

        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0].selector, "orders");
        assert_eq!(topics[1].display_name, "Payments");
        assert_eq!(topics[1].partitions, 6);
    }

    #[test]
    fn validates_required_bootstrap_servers() {
        let error =
            validate_config(&json!({ "topics": ["orders"] })).expect_err("validation should fail");
        assert!(error.contains("bootstrap_servers"));
    }

    #[test]
    fn finds_configured_topic() {
        let topic = find_topic(
            &json!({
                "bootstrap_servers": "broker-a:9092",
                "topics": [
                    {
                        "selector": "orders",
                        "sample_messages": [{ "order_id": "ord-1" }]
                    }
                ]
            }),
            "orders",
        )
        .expect("topic should exist");

        assert_eq!(topic.selector, "orders");
        assert_eq!(topic.sample_messages, vec![json!({ "order_id": "ord-1" })]);
    }
}
