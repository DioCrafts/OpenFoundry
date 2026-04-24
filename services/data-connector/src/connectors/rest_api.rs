//! REST API connector — reads data from HTTP endpoints.

use std::time::Instant;

use reqwest::{
    Client, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};

use super::{ConnectionTestResult, SyncPayload};

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("base_url").is_none() {
        return Err("rest_api connector requires 'base_url'".to_string());
    }
    Ok(())
}

pub async fn test_connection(config: &Value) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;

    let client = Client::new();
    let started = Instant::now();
    let url = build_url(config, None, true)?;
    let mut request = client.get(url.clone());
    request = apply_headers(request, config)?;

    let response = request.send().await.map_err(|error| error.to_string())?;
    let latency_ms = started.elapsed().as_millis();
    if !response.status().is_success() {
        return Err(format!("REST source returned HTTP {}", response.status()));
    }

    let body = response.text().await.unwrap_or_default();
    Ok(ConnectionTestResult {
        success: true,
        message: format!("GET {} returned HTTP 200", url.path()),
        latency_ms,
        details: Some(json!({
            "path": url.path(),
            "response_bytes": body.len(),
        })),
    })
}

pub async fn fetch_dataset(config: &Value, selector: &str) -> Result<SyncPayload, String> {
    validate_config(config)?;

    let client = Client::new();
    let url = build_url(config, Some(selector), false)?;
    let mut request = client.get(url.clone());
    request = apply_headers(request, config)?;

    let response = request.send().await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("REST source returned HTTP {}", response.status()));
    }

    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    let normalized = normalize_records(payload);
    let rows_synced = normalized
        .as_array()
        .map(|rows| rows.len() as i64)
        .unwrap_or(0);

    Ok(SyncPayload {
        bytes: serde_json::to_vec(&normalized).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced,
        file_name: file_name(selector),
        metadata: json!({
            "url": url.as_str(),
            "rows": rows_synced,
        }),
    })
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

fn apply_headers(
    mut request: reqwest::RequestBuilder,
    config: &Value,
) -> Result<reqwest::RequestBuilder, String> {
    if let Some(token) = config.get("bearer_token").and_then(Value::as_str) {
        request = request.bearer_auth(token);
    }

    let mut headers = HeaderMap::new();
    if let Some(header_map) = config.get("headers").and_then(Value::as_object) {
        for (name, value) in header_map {
            let header_name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?;
            let header_value = HeaderValue::from_str(
                value
                    .as_str()
                    .ok_or_else(|| format!("header '{name}' must be a string"))?,
            )
            .map_err(|error| error.to_string())?;
            headers.insert(header_name, header_value);
        }
    }

    Ok(request.headers(headers))
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
    }
}
