//! JSON connector — reads JSON/NDJSON from URLs or storage paths.

use std::time::Instant;

use serde_json::{Value, json};
use tokio::fs;

use super::{ConnectionTestResult, SyncPayload, add_source_signature, virtual_table_response};
use crate::{
    AppState,
    models::registration::{VirtualTableQueryRequest, VirtualTableQueryResponse},
};

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("url").is_none() && config.get("path").is_none() {
        return Err("json connector requires either 'url' or 'path'".to_string());
    }
    Ok(())
}

pub async fn test_connection(
    state: &AppState,
    config: &Value,
) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;
    let started = Instant::now();
    let bytes = read_source(state, config).await?;
    let row_count = count_rows(&bytes)?;

    Ok(ConnectionTestResult {
        success: true,
        message: "JSON source reachable".to_string(),
        latency_ms: started.elapsed().as_millis(),
        details: Some(json!({
            "bytes": bytes.len(),
            "rows": row_count,
            "source": source_label(config),
        })),
    })
}

pub async fn fetch_dataset(
    state: &AppState,
    config: &Value,
    selector: &str,
) -> Result<SyncPayload, String> {
    validate_config(config)?;
    let bytes = read_source(state, config).await?;
    let row_count = count_rows(&bytes)?;

    let mut payload = SyncPayload {
        bytes,
        format: "json".to_string(),
        rows_synced: row_count,
        file_name: file_name(config, selector, "json"),
        metadata: json!({
            "source": source_label(config),
            "rows": row_count,
        }),
    };
    add_source_signature(&mut payload);
    Ok(payload)
}

pub async fn query_virtual_table(
    state: &AppState,
    config: &Value,
    request: &VirtualTableQueryRequest,
) -> Result<VirtualTableQueryResponse, String> {
    let bytes = read_source(state, config).await?;
    let payload = serde_json::from_slice::<Value>(&bytes).map_err(|error| error.to_string())?;
    let rows = if let Some(array) = payload.as_array() {
        array.clone()
    } else {
        vec![payload]
    }
    .into_iter()
    .take(request.limit.unwrap_or(50).clamp(1, 500))
    .collect::<Vec<_>>();

    Ok(virtual_table_response(
        &request.selector,
        rows,
        json!({
            "source": source_label(config),
        }),
    ))
}

async fn read_source(state: &AppState, config: &Value) -> Result<Vec<u8>, String> {
    if let Some(path) = config.get("path").and_then(Value::as_str) {
        return fs::read(path).await.map_err(|error| error.to_string());
    }

    let url = config
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "json connector requires either 'url' or 'path'".to_string())?;
    let response = super::http_runtime::get(
        state,
        config,
        reqwest::Url::parse(url).map_err(|error| error.to_string())?,
        super::http_runtime::header_map(config)?,
        config
            .get("bearer_token")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
        None,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!("JSON source returned HTTP {}", response.status));
    }
    Ok(response.bytes)
}

fn count_rows(bytes: &[u8]) -> Result<i64, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    if trimmed.starts_with('[') {
        return serde_json::from_slice::<Value>(bytes)
            .map_err(|error| error.to_string())
            .map(|value| value.as_array().map(|rows| rows.len() as i64).unwrap_or(0));
    }

    if trimmed.starts_with('{') {
        return Ok(1);
    }

    Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as i64)
}

fn file_name(config: &Value, selector: &str, fallback_ext: &str) -> String {
    let candidate = config
        .get("path")
        .and_then(Value::as_str)
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .filter(|value| !value.is_empty());

    candidate.unwrap_or_else(|| {
        let stem = selector
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>();
        format!(
            "{}.{}",
            stem.trim_matches('_').if_empty_then("json_sync"),
            fallback_ext
        )
    })
}

fn source_label(config: &Value) -> String {
    config
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            config
                .get("url")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "json".to_string())
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for &str {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self.to_string()
        }
    }
}
