use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DatasetSchema {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub fields: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
