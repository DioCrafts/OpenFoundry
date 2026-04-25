//! Salesforce connector — validates an org token and runs SOQL queries for sync.

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
    let required = ["instance_url", "access_token"];
    for field in required {
        if config.get(field).is_none() {
            return Err(format!("salesforce connector requires '{field}'"));
        }
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
    let url = api_base_url(config)?
        .join("limits")
        .map_err(|error| error.to_string())?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        http_runtime::header_map(config)?,
        Some(access_token(config)?.to_string()),
        agent_url,
    )
    .await?;
    let latency_ms = started.elapsed().as_millis();

    if !(200..300).contains(&response.status) {
        return Err(format!("Salesforce returned HTTP {}", response.status));
    }

    let payload = http_runtime::json_body(&response)?;
    Ok(ConnectionTestResult {
        success: true,
        message: "Salesforce org reachable".to_string(),
        latency_ms,
        details: Some(json!({
            "instance_url": config.get("instance_url").and_then(Value::as_str).unwrap_or_default(),
            "api_version": api_version(config),
            "limits_keys": payload.as_object().map(|object| object.len()).unwrap_or(0),
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

    let soql = soql_query(config, selector)?;
    let mut url = api_base_url(config)?
        .join("query")
        .map_err(|error| error.to_string())?;
    url.query_pairs_mut().append_pair("q", &soql);

    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        http_runtime::header_map(config)?,
        Some(access_token(config)?.to_string()),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!("Salesforce returned HTTP {}", response.status));
    }

    let payload = http_runtime::json_body(&response)?;
    let records = payload
        .get("records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|mut record| {
            if let Value::Object(object) = &mut record {
                object.remove("attributes");
            }
            record
        })
        .collect::<Vec<_>>();

    let mut sync_payload = SyncPayload {
        bytes: serde_json::to_vec(&records).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced: records.len() as i64,
        file_name: "salesforce.json".to_string(),
        metadata: json!({
            "query": soql,
            "total_size": payload.get("totalSize").and_then(Value::as_i64).unwrap_or(records.len() as i64),
            "url": url.as_str(),
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
    let url = api_base_url(config)?
        .join("sobjects")
        .map_err(|error| error.to_string())?;
    let response = http_runtime::get(
        state,
        config,
        url,
        http_runtime::header_map(config)?,
        Some(access_token(config)?.to_string()),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "Salesforce catalog returned HTTP {}",
            response.status
        ));
    }

    let payload = http_runtime::json_body(&response)?;
    Ok(payload
        .get("sobjects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let name = item.get("name").and_then(Value::as_str)?.to_string();
            let label = item
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(&name)
                .to_string();
            Some(basic_discovered_source(
                name.clone(),
                label,
                "salesforce_object",
                item,
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

fn access_token(config: &Value) -> Result<&str, String> {
    config
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "salesforce connector requires 'access_token'".to_string())
}

fn api_base_url(config: &Value) -> Result<Url, String> {
    let instance_url = config
        .get("instance_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "salesforce connector requires 'instance_url'".to_string())?;
    let base = Url::parse(instance_url).map_err(|error| error.to_string())?;
    base.join(&format!("/services/data/{}/", api_version(config)))
        .map_err(|error| error.to_string())
}

fn api_version(config: &Value) -> &str {
    config
        .get("api_version")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("v60.0")
}

fn row_limit(config: &Value) -> i64 {
    config
        .get("row_limit")
        .and_then(Value::as_i64)
        .unwrap_or(200)
        .clamp(1, 2_000)
}

fn soql_query(config: &Value, selector: &str) -> Result<String, String> {
    if selector.trim().to_ascii_lowercase().starts_with("select ") {
        return Ok(selector.trim().to_string());
    }

    if !selector.trim().is_empty() {
        return Ok(format!(
            "SELECT Id, Name FROM {} LIMIT {}",
            selector.trim(),
            row_limit(config)
        ));
    }

    config
        .get("query")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "salesforce sync requires a SOQL query or object selector".to_string())
}
