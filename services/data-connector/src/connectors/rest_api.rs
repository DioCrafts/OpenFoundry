//! REST API connector — reads data from HTTP endpoints.

use std::time::Instant;

use reqwest::{Url, header::HeaderMap};
use serde_json::{Value, json};

use super::{
    ConnectionTestResult, SyncPayload, add_source_signature, basic_discovered_source, http_runtime,
    virtual_table_response,
};
use crate::{
    AppState,
    models::registration::{DiscoveredSource, VirtualTableQueryRequest, VirtualTableQueryResponse},
};

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("base_url").is_none() {
        return Err("rest_api connector requires 'base_url'".to_string());
    }
    Ok(())
}

pub async fn test_connection(
    state: &AppState,
    config: &Value,
    agent_url: Option<&str>,
) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;

    let started = Instant::now();
    let url = build_url(config, None, true)?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        build_headers(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    let latency_ms = started.elapsed().as_millis();
    if !(200..300).contains(&response.status) {
        return Err(format!("REST source returned HTTP {}", response.status));
    }

    Ok(ConnectionTestResult {
        success: true,
        message: format!("GET {} returned HTTP {}", url.path(), response.status),
        latency_ms,
        details: Some(json!({
            "path": url.path(),
            "response_bytes": response.bytes.len(),
            "agent_url": agent_url,
        })),
    })
}

pub async fn fetch_dataset(
    state: &AppState,
    config: &Value,
    selector: &str,
    agent_url: Option<&str>,
) -> Result<SyncPayload, String> {
    validate_config(config)?;

    let url = build_url(config, Some(selector), false)?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        build_headers(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!("REST source returned HTTP {}", response.status));
    }

    let payload = http_runtime::json_body(&response)?;
    let normalized = normalize_records(payload);
    let rows_synced = normalized
        .as_array()
        .map(|rows| rows.len() as i64)
        .unwrap_or(0);

    let mut sync_payload = SyncPayload {
        bytes: serde_json::to_vec(&normalized).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced,
        file_name: file_name(selector),
        metadata: json!({
            "url": url.as_str(),
            "rows": rows_synced,
            "headers": response.headers,
            "agent_url": agent_url,
        }),
    };
    add_source_signature(&mut sync_payload);
    Ok(sync_payload)
}

pub async fn discover_sources(
    state: &AppState,
    config: &Value,
    agent_url: Option<&str>,
) -> Result<Vec<DiscoveredSource>, String> {
    validate_config(config)?;
    if let Some(resources) = config.get("resources").and_then(Value::as_array) {
        return Ok(resources
            .iter()
            .filter_map(discovered_from_config)
            .collect());
    }

    if let Some(catalog_path) = config.get("catalog_path").and_then(Value::as_str) {
        let url = build_url(config, Some(catalog_path), false)?;
        let response = http_runtime::get(
            state,
            config,
            url,
            build_headers(config)?,
            bearer_token(config),
            agent_url,
        )
        .await?;
        if !(200..300).contains(&response.status) {
            return Err(format!("REST catalog returned HTTP {}", response.status));
        }
        let payload = http_runtime::json_body(&response)?;
        let entries = normalize_records(payload)
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| discovered_from_config(&entry))
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            return Ok(entries);
        }
    }

    Ok(vec![basic_discovered_source(
        config
            .get("resource_path")
            .and_then(Value::as_str)
            .unwrap_or("/"),
        config
            .get("resource_name")
            .and_then(Value::as_str)
            .unwrap_or("REST resource"),
        "rest_resource",
        json!({
            "base_url": config.get("base_url").and_then(Value::as_str),
        }),
    )])
}

pub async fn query_virtual_table(
    state: &AppState,
    config: &Value,
    request: &VirtualTableQueryRequest,
    agent_url: Option<&str>,
) -> Result<VirtualTableQueryResponse, String> {
    let payload = fetch_dataset(state, config, &request.selector, agent_url).await?;
    let rows = serde_json::from_slice::<Value>(&payload.bytes)
        .map_err(|error| error.to_string())?
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(request.limit.unwrap_or(50).clamp(1, 500))
        .collect::<Vec<_>>();

    Ok(virtual_table_response(
        &request.selector,
        rows,
        payload.metadata,
    ))
}

fn build_url(config: &Value, selector: Option<&str>, for_health: bool) -> Result<Url, String> {
    let base_url = config
        .get("base_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "rest_api connector requires 'base_url'".to_string())?;
    let base = Url::parse(base_url).map_err(|error| error.to_string())?;

    let path = if for_health {
        config
            .get("health_path")
            .and_then(Value::as_str)
            .or_else(|| config.get("resource_path").and_then(Value::as_str))
            .or_else(|| selector.filter(|value| !value.trim().is_empty()))
            .unwrap_or("/health")
    } else {
        selector
            .filter(|value| !value.trim().is_empty())
            .or_else(|| config.get("resource_path").and_then(Value::as_str))
            .unwrap_or("/")
    };

    base.join(path).map_err(|error| error.to_string())
}

fn build_headers(config: &Value) -> Result<HeaderMap, String> {
    http_runtime::header_map(config)
}

fn bearer_token(config: &Value) -> Option<String> {
    config
        .get("bearer_token")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn normalize_records(payload: Value) -> Value {
    match payload {
        Value::Array(rows) => Value::Array(rows),
        Value::Object(mut object) => {
            if let Some(records) = object.remove("data").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("items").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("records").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("value").and_then(array_if_any) {
                Value::Array(records)
            } else {
                Value::Array(vec![Value::Object(object)])
            }
        }
        other => Value::Array(vec![json!({ "value": other })]),
    }
}

fn array_if_any(value: Value) -> Option<Vec<Value>> {
    value.as_array().cloned()
}

fn discovered_from_config(value: &Value) -> Option<DiscoveredSource> {
    let selector = value
        .get("selector")
        .or_else(|| value.get("path"))
        .and_then(Value::as_str)?
        .to_string();
    let display_name = value
        .get("display_name")
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or(&selector)
        .to_string();
    Some(basic_discovered_source(
        selector,
        display_name,
        "rest_resource",
        value.clone(),
    ))
}

fn file_name(selector: &str) -> String {
    let stem = selector
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    format!("{}.json", stem.if_empty_then("rest_sync"))
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::normalize_records;

    #[test]
    fn normalizes_common_rest_wrappers() {
        assert_eq!(
            normalize_records(json!({ "data": [{ "id": 1 }, { "id": 2 }] })),
            json!([{ "id": 1 }, { "id": 2 }])
        );
        assert_eq!(
            normalize_records(json!({ "status": "ok" })),
            json!([{ "status": "ok" }])
        );
        assert_eq!(
            normalize_records(json!({ "value": [{ "asset": "pump-01" }] })),
            json!([{ "asset": "pump-01" }])
        );
    }
}
