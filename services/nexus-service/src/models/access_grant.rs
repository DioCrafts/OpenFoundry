use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::decode_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessGrant {
    pub id: uuid::Uuid,
    pub share_id: uuid::Uuid,
    pub peer_id: uuid::Uuid,
    pub query_template: String,
    pub max_rows_per_query: i64,
    pub can_replicate: bool,
    pub allowed_purposes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccessGrantRow {
    pub id: uuid::Uuid,
    pub share_id: uuid::Uuid,
    pub peer_id: uuid::Uuid,
    pub query_template: String,
    pub max_rows_per_query: i64,
    pub can_replicate: bool,
    pub allowed_purposes: Value,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}

impl TryFrom<AccessGrantRow> for AccessGrant {
    type Error = String;

    fn try_from(row: AccessGrantRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            share_id: row.share_id,
            peer_id: row.peer_id,
            query_template: row.query_template,
            max_rows_per_query: row.max_rows_per_query,
            can_replicate: row.can_replicate,
            allowed_purposes: decode_json(row.allowed_purposes, "allowed_purposes")?,
            expires_at: row.expires_at,
            issued_at: row.issued_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FederatedQueryRequest {
    pub share_id: uuid::Uuid,
    pub sql: String,
    pub purpose: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedQueryResult {
    pub share_id: uuid::Uuid,
    pub dataset_name: String,
    pub source_peer: String,
    pub executed_sql: String,
    pub query_mode: String,
    pub limit: usize,
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
}
