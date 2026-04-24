use serde::Serialize;
use serde_json::Value;

pub mod csv;
pub mod json;
pub mod postgres;
pub mod rest_api;
pub mod salesforce;

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
