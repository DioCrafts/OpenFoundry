//! Salesforce connector — validates an org token and runs SOQL queries for sync.

use std::time::Instant;

use reqwest::{Client, Url};
use serde_json::{Value, json};

use super::{ConnectionTestResult, SyncPayload};

pub fn validate_config(config: &Value) -> Result<(), String> {
    let required = ["instance_url", "access_token"];
    for field in required {
        if config.get(field).is_none() {
            return Err(format!("salesforce connector requires '{field}'"));
        }
    }
    Ok(())
}

pub async fn test_connection(config: &Value) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;

    let client = Client::new();
    let started = Instant::now();
    let url = api_base_url(config)?
        .join("limits")
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url.clone())
        .bearer_auth(access_token(config)?)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let latency_ms = started.elapsed().as_millis();

    if !response.status().is_success() {
        return Err(format!("Salesforce returned HTTP {}", response.status()));
    }

    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    Ok(ConnectionTestResult {
        success: true,
        message: "Salesforce org reachable".to_string(),
        latency_ms,
        details: Some(json!({
            "instance_url": config.get("instance_url").and_then(Value::as_str).unwrap_or_default(),
            "api_version": api_version(config),
            "limits_keys": payload.as_object().map(|object| object.len()).unwrap_or(0),
        })),
    })
}

pub async fn fetch_dataset(config: &Value, selector: &str) -> Result<SyncPayload, String> {
    validate_config(config)?;

    let soql = if selector.trim().to_ascii_lowercase().starts_with("select ") {
        selector.trim().to_string()
    } else if !selector.trim().is_empty() {
        format!(
            "SELECT Id FROM {} LIMIT {}",
            selector.trim(),
            row_limit(config)
        )
    } else if let Some(default_query) = config.get("query").and_then(Value::as_str) {
        default_query.trim().to_string()
    } else {
        return Err("salesforce sync requires a SOQL query or object selector".to_string());
    };

    let mut url = api_base_url(config)?
        .join("query")
        .map_err(|error| error.to_string())?;
    url.query_pairs_mut().append_pair("q", &soql);

    let client = Client::new();
    let response = client
        .get(url.clone())
        .bearer_auth(access_token(config)?)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Salesforce returned HTTP {}", response.status()));
    }

    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
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

    Ok(SyncPayload {
        bytes: serde_json::to_vec(&records).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced: records.len() as i64,
        file_name: "salesforce.json".to_string(),
        metadata: json!({
            "query": soql,
            "total_size": payload.get("totalSize").and_then(Value::as_i64).unwrap_or(records.len() as i64),
            "url": url.as_str(),
        }),
    })
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
