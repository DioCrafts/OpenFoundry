use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDeadLetter {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub payload: Value,
    pub event_time: DateTime<Utc>,
    pub reason: String,
    pub validation_errors: Vec<String>,
    pub status: String,
    pub replay_count: i32,
    pub last_replayed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayDeadLetterRequest {
    pub payload: Option<Value>,
    pub event_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayDeadLetterResponse {
    pub dead_letter: StreamingDeadLetter,
    pub replay_sequence_no: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct StreamingDeadLetterRow {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub payload: Value,
    pub event_time: DateTime<Utc>,
    pub reason: String,
    pub validation_errors: SqlJson<Vec<String>>,
    pub status: String,
    pub replay_count: i32,
    pub last_replayed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<StreamingDeadLetterRow> for StreamingDeadLetter {
    fn from(value: StreamingDeadLetterRow) -> Self {
        Self {
            id: value.id,
            stream_id: value.stream_id,
            payload: value.payload,
            event_time: value.event_time,
            reason: value.reason,
            validation_errors: value.validation_errors.0,
            status: value.status,
            replay_count: value.replay_count,
            last_replayed_at: value.last_replayed_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
