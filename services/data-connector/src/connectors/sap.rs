//! SAP OData connector — validates an OData service and syncs entity sets.

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
        return Err("sap connector requires 'base_url'".to_string());
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
    let url = service_root(config)?;
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
        return Err(format!("SAP service returned HTTP {}", response.status));
    }

    Ok(ConnectionTestResult {
        success: true,
        message: "SAP OData service reachable".to_string(),
        latency_ms,
        details: Some(json!({
            "service_root": url.as_str(),
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
    let url = service_root(config)?
        .join(selector)
        .map_err(|error| error.to_string())?;
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
        return Err(format!("SAP service returned HTTP {}", response.status));
    }

    let rows = normalize_entity_rows(http_runtime::json_body(&response)?);
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
    if let Some(entities) = config.get("entities").and_then(Value::as_array) {
        return Ok(entities
            .iter()
            .filter_map(|entity| {
                let selector = entity
                    .get("selector")
                    .or_else(|| entity.get("name"))
                    .and_then(Value::as_str)?
                    .to_string();
                let display_name = entity
                    .get("display_name")
                    .or_else(|| entity.get("label"))
                    .and_then(Value::as_str)
                    .unwrap_or(&selector)
                    .to_string();
                Some(basic_discovered_source(
                    selector,
                    display_name,
                    "sap_entity_set",
                    entity.clone(),
                ))
            })
            .collect());
    }

    let url = service_root(config)?;
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
        return Err(format!("SAP service returned HTTP {}", response.status));
    }

    let payload = http_runtime::json_body(&response)?;
    let entries = payload
        .get("d")
        .and_then(|value| value.get("EntitySets"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|value| value.to_string()))
        .collect::<Vec<_>>();
    if !entries.is_empty() {
        return Ok(entries
            .into_iter()
            .map(|selector| {
                basic_discovered_source(selector.clone(), selector, "sap_entity_set", json!({}))
            })
            .collect());
    }

    Ok(payload
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let selector = entry
                .get("url")
                .or_else(|| entry.get("name"))
                .and_then(Value::as_str)?
                .to_string();
            let display_name = entry
                .get("title")
                .or_else(|| entry.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(&selector)
                .to_string();
            Some(basic_discovered_source(
                selector,
                display_name,
                "sap_entity_set",
                entry,
            ))
        })
        .collect())
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

fn service_root(config: &Value) -> Result<Url, String> {
    let base_url = config
        .get("base_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "sap connector requires 'base_url'".to_string())?;
    let base = Url::parse(base_url).map_err(|error| error.to_string())?;
    let service_path = config
        .get("service_path")
        .and_then(Value::as_str)
        .unwrap_or("/");
    base.join(service_path).map_err(|error| error.to_string())
}

fn bearer_token(config: &Value) -> Option<String> {
    config
        .get("bearer_token")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn normalize_entity_rows(payload: Value) -> Vec<Value> {
    if let Some(rows) = payload
        .get("d")
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
    {
        return rows.clone();
    }
    if let Some(rows) = payload.get("value").and_then(Value::as_array) {
        return rows.clone();
    }
    payload.as_array().cloned().unwrap_or_else(|| vec![payload])
}
