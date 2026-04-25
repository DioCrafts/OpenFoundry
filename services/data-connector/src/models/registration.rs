use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConnectionRegistration {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub selector: String,
    pub display_name: String,
    pub source_kind: String,
    pub registration_mode: String,
    pub auto_sync: bool,
    pub update_detection: bool,
    pub target_dataset_id: Option<Uuid>,
    pub last_source_signature: Option<String>,
    pub last_dataset_version: Option<i32>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredSource {
    pub selector: String,
    pub display_name: String,
    pub source_kind: String,
    pub supports_sync: bool,
    pub supports_zero_copy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_signature: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AutoRegisterRequest {
    #[serde(default)]
    pub selectors: Vec<String>,
    pub registration_mode: Option<String>,
    pub auto_sync: Option<bool>,
    pub update_detection: Option<bool>,
    pub default_target_dataset_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct BulkRegisterRequest {
    pub registrations: Vec<BulkRegistrationItem>,
}

#[derive(Debug, Deserialize)]
pub struct BulkRegistrationItem {
    pub selector: String,
    pub display_name: Option<String>,
    pub source_kind: Option<String>,
    pub registration_mode: Option<String>,
    pub auto_sync: Option<bool>,
    pub update_detection: Option<bool>,
    pub target_dataset_id: Option<Uuid>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct VirtualTableQueryRequest {
    pub selector: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTableQueryResponse {
    pub selector: String,
    pub mode: String,
    pub columns: Vec<String>,
    pub row_count: usize,
    pub rows: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_signature: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
