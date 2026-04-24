use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::decode_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingContract {
    pub id: uuid::Uuid,
    pub peer_id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub dataset_locator: String,
    pub allowed_purposes: Vec<String>,
    pub data_classes: Vec<String>,
    pub residency_region: String,
    pub query_template: String,
    pub max_rows_per_query: i64,
    pub replication_mode: String,
    pub encryption_profile: String,
    pub retention_days: i32,
    pub status: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContractRow {
    pub id: uuid::Uuid,
    pub peer_id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub dataset_locator: String,
    pub allowed_purposes: Value,
    pub data_classes: Value,
    pub residency_region: String,
    pub query_template: String,
    pub max_rows_per_query: i64,
    pub replication_mode: String,
    pub encryption_profile: String,
    pub retention_days: i32,
    pub status: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ContractRow> for SharingContract {
    type Error = String;

    fn try_from(row: ContractRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            peer_id: row.peer_id,
            name: row.name,
            description: row.description,
            dataset_locator: row.dataset_locator,
            allowed_purposes: decode_json(row.allowed_purposes, "allowed_purposes")?,
            data_classes: decode_json(row.data_classes, "data_classes")?,
            residency_region: row.residency_region,
            query_template: row.query_template,
            max_rows_per_query: row.max_rows_per_query,
            replication_mode: row.replication_mode,
            encryption_profile: row.encryption_profile,
            retention_days: row.retention_days,
            status: row.status,
            signed_at: row.signed_at,
            expires_at: row.expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateContractRequest {
    pub peer_id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub dataset_locator: String,
    #[serde(default)]
    pub allowed_purposes: Vec<String>,
    #[serde(default)]
    pub data_classes: Vec<String>,
    pub residency_region: String,
    pub query_template: String,
    pub max_rows_per_query: i64,
    pub replication_mode: String,
    pub encryption_profile: String,
    pub retention_days: i32,
    pub status: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateContractRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub dataset_locator: Option<String>,
    pub allowed_purposes: Option<Vec<String>>,
    pub data_classes: Option<Vec<String>>,
    pub residency_region: Option<String>,
    pub query_template: Option<String>,
    pub max_rows_per_query: Option<i64>,
    pub replication_mode: Option<String>,
    pub encryption_profile: Option<String>,
    pub retention_days: Option<i32>,
    pub status: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}
