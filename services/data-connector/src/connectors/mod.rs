use serde::Serialize;
use serde_json::Value;

use sha2::{Digest, Sha256};

use crate::models::registration::{DiscoveredSource, VirtualTableQueryResponse};

pub mod csv;
pub mod http_runtime;
pub mod iot;
pub mod json;
pub mod postgres;
pub mod rest_api;
pub mod salesforce;
pub mod sap;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SyncPayload {
    pub bytes: Vec<u8>,
    pub format: String,
    pub rows_synced: i64,
    pub file_name: String,
    pub metadata: Value,
}

pub fn add_source_signature(payload: &mut SyncPayload) {
    let signature = source_signature(&payload.bytes);
    if let Some(metadata) = payload.metadata.as_object_mut() {
        metadata.insert("source_signature".to_string(), Value::String(signature));
    }
}

pub fn source_signature(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{:x}", digest)
}

pub fn virtual_table_response(
    selector: &str,
    rows: Vec<Value>,
    metadata: Value,
) -> VirtualTableQueryResponse {
    let row_count = rows.len();
    let columns = rows
        .iter()
        .find_map(|row| {
            row.as_object()
                .map(|object| object.keys().cloned().collect())
        })
        .unwrap_or_default();
    let source_signature = serde_json::to_vec(&rows)
        .ok()
        .map(|bytes| source_signature(&bytes));
    VirtualTableQueryResponse {
        selector: selector.to_string(),
        mode: "zero_copy".to_string(),
        columns,
        row_count,
        rows,
        source_signature,
        metadata,
    }
}

pub fn basic_discovered_source(
    selector: impl Into<String>,
    display_name: impl Into<String>,
    source_kind: impl Into<String>,
    metadata: Value,
) -> DiscoveredSource {
    DiscoveredSource {
        selector: selector.into(),
        display_name: display_name.into(),
        source_kind: source_kind.into(),
        supports_sync: true,
        supports_zero_copy: true,
        source_signature: None,
        metadata,
    }
}
