use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub connector_type: String,
    pub config: serde_json::Value,
    pub status: String,
    pub owner_id: Uuid,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub connector_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ListConnectionsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Supported connector types for validation.
pub const VALID_TYPES: &[&str] = &[
    "postgresql",
    "mysql",
    "csv",
    "parquet",
    "json",
    "s3",
    "rest_api",
    "salesforce",
    "sap",
    "iot",
];
