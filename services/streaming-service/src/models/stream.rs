use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamField {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub semantic_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSchema {
    pub fields: Vec<StreamField>,
    pub primary_key: Option<String>,
    pub watermark_field: Option<String>,
}

impl Default for StreamSchema {
    fn default() -> Self {
        Self {
            fields: vec![
                StreamField {
                    name: "event_time".to_string(),
                    data_type: "timestamp".to_string(),
                    nullable: false,
                    semantic_role: "event_time".to_string(),
                },
                StreamField {
                    name: "customer_id".to_string(),
                    data_type: "string".to_string(),
                    nullable: false,
                    semantic_role: "join_key".to_string(),
                },
            ],
            primary_key: None,
            watermark_field: Some("event_time".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorBinding {
    pub connector_type: String,
    pub endpoint: String,
    pub format: String,
    pub config: Value,
}

impl Default for ConnectorBinding {
    fn default() -> Self {
        Self {
            connector_type: "kafka".to_string(),
            endpoint: "kafka://stream/orders".to_string(),
            format: "json".to_string(),
            config: serde_json::json!({ "compression": "snappy" }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub schema: StreamSchema,
    pub source_binding: ConnectorBinding,
    pub retention_hours: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventInput {
    pub payload: Value,
    pub event_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PushStreamEventsRequest {
    pub events: Vec<StreamEventInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PushStreamEventsResponse {
    pub stream_id: Uuid,
    pub accepted_events: usize,
    pub dead_lettered_events: usize,
    pub first_sequence_no: Option<i64>,
    pub last_sequence_no: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateStreamRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub schema: Option<StreamSchema>,
    pub source_binding: Option<ConnectorBinding>,
    pub retention_hours: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStreamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub schema: Option<StreamSchema>,
    pub source_binding: Option<ConnectorBinding>,
    pub retention_hours: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StreamRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub schema: SqlJson<StreamSchema>,
    pub source_binding: SqlJson<ConnectorBinding>,
    pub retention_hours: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<StreamRow> for StreamDefinition {
    fn from(value: StreamRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            schema: value.schema.0,
            source_binding: value.source_binding.0,
            retention_hours: value.retention_hours,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
