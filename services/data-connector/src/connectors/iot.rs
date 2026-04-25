//! IoT / IIoT connector — polls feed endpoints that expose JSON events.

use std::time::Instant;

use reqwest::Url;
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
        return Err("iot connector requires 'base_url'".to_string());
    }
    if config.get("feed_path").is_none() && config.get("feeds").is_none() {
        return Err("iot connector requires 'feed_path' or 'feeds'".to_string());
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
    let selector = default_selector(config)?;
    let url = build_feed_url(config, &selector)?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        http_runtime::header_map(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    let latency_ms = started.elapsed().as_millis();
    if !(200..300).contains(&response.status) {
        return Err(format!("IoT source returned HTTP {}", response.status));
    }

    Ok(ConnectionTestResult {
        success: true,
        message: "IoT feed reachable".to_string(),
        latency_ms,
        details: Some(json!({
            "feed": selector,
            "url": url.as_str(),
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
    let url = build_feed_url(config, selector)?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        http_runtime::header_map(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!("IoT source returned HTTP {}", response.status));
    }

    let rows = normalize_rows(http_runtime::json_body(&response)?);
    let mut payload = SyncPayload {
        bytes: serde_json::to_vec(&rows).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced: rows.len() as i64,
        file_name: format!("{}.json", selector.replace('/', "_")),
        metadata: json!({
            "selector": selector,
            "url": url.as_str(),
            "headers": response.headers,
            "agent_url": agent_url,
        }),
    };
    add_source_signature(&mut payload);
    Ok(payload)
}

pub async fn discover_sources(
    state: &AppState,
    config: &Value,
    agent_url: Option<&str>,
) -> Result<Vec<DiscoveredSource>, String> {
    validate_config(config)?;
    if let Some(feeds) = config.get("feeds").and_then(Value::as_array) {
        return Ok(feeds
            .iter()
            .filter_map(|feed| {
                let selector = feed
                    .get("selector")
                    .or_else(|| feed.get("path"))
                    .and_then(Value::as_str)?
                    .to_string();
                let display_name = feed
                    .get("display_name")
                    .or_else(|| feed.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(&selector)
                    .to_string();
                Some(basic_discovered_source(
                    selector,
                    display_name,
                    "iot_feed",
                    feed.clone(),
                ))
            })
            .collect());
    }

    if let Some(catalog_path) = config.get("catalog_path").and_then(Value::as_str) {
        let url = build_feed_url(config, catalog_path)?;
        let response = http_runtime::get(
            state,
            config,
            url,
            http_runtime::header_map(config)?,
            bearer_token(config),
            agent_url,
        )
        .await?;
        if !(200..300).contains(&response.status) {
            return Err(format!("IoT catalog returned HTTP {}", response.status));
        }
        let payload = http_runtime::json_body(&response)?;
        return Ok(normalize_rows(payload)
            .into_iter()
            .filter_map(|entry| {
                let selector = entry
                    .get("selector")
                    .or_else(|| entry.get("path"))
                    .and_then(Value::as_str)?
                    .to_string();
                let display_name = entry
                    .get("display_name")
                    .or_else(|| entry.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(&selector)
                    .to_string();
                Some(basic_discovered_source(
                    selector,
                    display_name,
                    "iot_feed",
                    entry,
                ))
            })
            .collect());
    }

    let selector = default_selector(config)?;
    Ok(vec![basic_discovered_source(
        selector.clone(),
        selector,
        "iot_feed",
        json!({}),
    )])
}

pub async fn query_virtual_table(
    state: &AppState,
    config: &Value,
    request: &VirtualTableQueryRequest,
    agent_url: Option<&str>,
) -> Result<VirtualTableQueryResponse, String> {
    let payload = fetch_dataset(state, config, &request.selector, agent_url).await?;
    let rows = serde_json::from_slice::<Vec<Value>>(&payload.bytes)
        .map_err(|error| error.to_string())?
        .into_iter()
        .take(request.limit.unwrap_or(50).clamp(1, 500))
        .collect::<Vec<_>>();
    Ok(virtual_table_response(
        &request.selector,
        rows,
        payload.metadata,
    ))
}

fn build_feed_url(config: &Value, selector: &str) -> Result<Url, String> {
    let base_url = config
        .get("base_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "iot connector requires 'base_url'".to_string())?;
    let base = Url::parse(base_url).map_err(|error| error.to_string())?;
    base.join(selector).map_err(|error| error.to_string())
}

fn default_selector(config: &Value) -> Result<String, String> {
    config
        .get("feed_path")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .or_else(|| {
            config
                .get("feeds")
                .and_then(Value::as_array)
                .and_then(|feeds| feeds.first())
                .and_then(|feed| {
                    feed.get("selector")
                        .or_else(|| feed.get("path"))
                        .and_then(Value::as_str)
                })
                .map(|value| value.to_string())
        })
        .ok_or_else(|| "iot connector requires at least one feed selector".to_string())
}

fn bearer_token(config: &Value) -> Option<String> {
    config
        .get("bearer_token")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn normalize_rows(payload: Value) -> Vec<Value> {
    payload
        .as_array()
        .cloned()
        .or_else(|| {
            payload
                .get("events")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| payload.get("data").and_then(Value::as_array).cloned())
        })
        .unwrap_or_else(|| vec![payload])
}
