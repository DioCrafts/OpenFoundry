use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{
    access_grant::AccessGrant,
    decode_json,
    sync_status::{EncryptionPosture, SchemaCompatibilityReport, SyncStatus},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDataset {
    pub id: uuid::Uuid,
    pub contract_id: uuid::Uuid,
    pub provider_peer_id: uuid::Uuid,
    pub consumer_peer_id: uuid::Uuid,
    pub provider_space_id: Option<uuid::Uuid>,
    pub consumer_space_id: Option<uuid::Uuid>,
    pub dataset_name: String,
    pub selector: Value,
    pub provider_schema: Value,
    pub consumer_schema: Value,
    pub sample_rows: Vec<Value>,
    pub replication_mode: String,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareDetail {
    pub share: SharedDataset,
    pub access_grant: Option<AccessGrant>,
    pub sync_status: Option<SyncStatus>,
    pub encryption: EncryptionPosture,
    pub compatibility: SchemaCompatibilityReport,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SharedDatasetRow {
    pub id: uuid::Uuid,
    pub contract_id: uuid::Uuid,
    pub provider_peer_id: uuid::Uuid,
    pub consumer_peer_id: uuid::Uuid,
    pub provider_space_id: Option<uuid::Uuid>,
    pub consumer_space_id: Option<uuid::Uuid>,
    pub dataset_name: String,
    pub selector: Value,
    pub provider_schema: Value,
    pub consumer_schema: Value,
    pub sample_rows: Value,
    pub replication_mode: String,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<SharedDatasetRow> for SharedDataset {
    type Error = String;

    fn try_from(row: SharedDatasetRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            contract_id: row.contract_id,
            provider_peer_id: row.provider_peer_id,
            consumer_peer_id: row.consumer_peer_id,
            provider_space_id: row.provider_space_id,
            consumer_space_id: row.consumer_space_id,
            dataset_name: row.dataset_name,
            selector: row.selector,
            provider_schema: row.provider_schema,
            consumer_schema: row.consumer_schema,
            sample_rows: decode_json(row.sample_rows, "sample_rows")?,
            replication_mode: row.replication_mode,
            status: row.status,
            last_sync_at: row.last_sync_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateShareRequest {
    pub contract_id: uuid::Uuid,
    pub provider_peer_id: uuid::Uuid,
    pub consumer_peer_id: uuid::Uuid,
    pub provider_space_id: Option<uuid::Uuid>,
    pub consumer_space_id: Option<uuid::Uuid>,
    pub dataset_name: String,
    #[serde(default)]
    pub selector: Value,
    pub provider_schema: Value,
    pub consumer_schema: Value,
    #[serde(default)]
    pub sample_rows: Vec<Value>,
    pub replication_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateShareRequest {
    pub dataset_name: Option<String>,
    pub provider_space_id: Option<uuid::Uuid>,
    pub consumer_space_id: Option<uuid::Uuid>,
    pub selector: Option<Value>,
    pub consumer_schema: Option<Value>,
    pub sample_rows: Option<Vec<Value>>,
    pub replication_mode: Option<String>,
    pub status: Option<String>,
}
