//! CSV connector — reads CSV files from URLs or storage paths.

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
        return Err("csv connector requires either 'url' or 'path'".to_string());
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
        message: "CSV source reachable".to_string(),
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
        format: "csv".to_string(),
        rows_synced: row_count,
        file_name: file_name(config, selector, "csv"),
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
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader
        .headers()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|header| header.to_string())
        .collect::<Vec<_>>();
    let rows = reader
        .records()
        .take(request.limit.unwrap_or(50).clamp(1, 500))
        .map(|record| {
            let record = record.map_err(|error| error.to_string())?;
            let mut object = serde_json::Map::new();
            for (index, header) in headers.iter().enumerate() {
                object.insert(
                    header.clone(),
                    Value::String(record.get(index).unwrap_or_default().to_string()),
                );
            }
            Ok(Value::Object(object))
        })
        .collect::<Result<Vec<_>, String>>()?;

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
        .ok_or_else(|| "csv connector requires either 'url' or 'path'".to_string())?;
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
        return Err(format!("CSV source returned HTTP {}", response.status));
    }
    Ok(response.bytes)
}

fn count_rows(bytes: &[u8]) -> Result<i64, String> {
    let mut reader = csv::Reader::from_reader(bytes);
    let mut total = 0_i64;
    for record in reader.records() {
        record.map_err(|error| error.to_string())?;
        total += 1;
    }
    Ok(total)
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
            stem.trim_matches('_').if_empty_then("csv_sync"),
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
        .unwrap_or_else(|| "csv".to_string())
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
