use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub id: uuid::Uuid,
    pub share_id: uuid::Uuid,
    pub mode: String,
    pub status: String,
    pub rows_replicated: i64,
    pub backlog_rows: i64,
    pub encrypted_in_transit: bool,
    pub encrypted_at_rest: bool,
    pub key_version: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub audit_cursor: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncStatusRow {
    pub id: uuid::Uuid,
    pub share_id: uuid::Uuid,
    pub mode: String,
    pub status: String,
    pub rows_replicated: i64,
    pub backlog_rows: i64,
    pub encrypted_in_transit: bool,
    pub encrypted_at_rest: bool,
    pub key_version: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub audit_cursor: String,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<SyncStatusRow> for SyncStatus {
    type Error = String;

    fn try_from(row: SyncStatusRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            share_id: row.share_id,
            mode: row.mode,
            status: row.status,
            rows_replicated: row.rows_replicated,
            backlog_rows: row.backlog_rows,
            encrypted_in_transit: row.encrypted_in_transit,
            encrypted_at_rest: row.encrypted_at_rest,
            key_version: row.key_version,
            last_sync_at: row.last_sync_at,
            next_sync_at: row.next_sync_at,
            audit_cursor: row.audit_cursor,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusOverview {
    pub peer_count: i64,
    pub active_peer_count: i64,
    pub contract_count: i64,
    pub active_contract_count: i64,
    pub private_space_count: i64,
    pub shared_space_count: i64,
    pub share_count: i64,
    pub federated_access_count: i64,
    pub encrypted_share_count: i64,
    pub replication_ready_count: i64,
    pub pending_schema_reviews: i64,
    pub audit_bridge_status: String,
    pub latest_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionPosture {
    pub share_id: uuid::Uuid,
    pub transport_cipher: String,
    pub at_rest_cipher: String,
    pub key_version: String,
    pub profile: String,
    pub encrypted_in_transit: bool,
    pub encrypted_at_rest: bool,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCompatibilityReport {
    pub share_id: uuid::Uuid,
    pub compatible: bool,
    pub missing_fields: Vec<String>,
    pub type_mismatches: Vec<String>,
    pub reviewed_at: DateTime<Utc>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationPlan {
    pub share_id: uuid::Uuid,
    pub dataset_name: String,
    pub mode: String,
    pub status: String,
    pub rows_replicated: i64,
    pub backlog_rows: i64,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub selective_filter: Value,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBridgeEntry {
    pub share_id: uuid::Uuid,
    pub dataset_name: String,
    pub peer_name: String,
    pub contract_name: String,
    pub audit_cursor: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub status: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBridgeSummary {
    pub bridge_status: String,
    pub entry_count: i64,
    pub latest_cursor: String,
    pub entries: Vec<AuditBridgeEntry>,
}
