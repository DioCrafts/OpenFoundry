use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenRecordProvenance {
    pub field: String,
    pub source: String,
    pub external_id: String,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenRecord {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub title: String,
    pub canonical_values: Value,
    pub provenance: Vec<GoldenRecordProvenance>,
    pub completeness_score: f32,
    pub confidence_score: f32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GoldenRecordRow {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub title: String,
    pub canonical_values: SqlJson<Value>,
    pub provenance: SqlJson<Vec<GoldenRecordProvenance>>,
    pub completeness_score: f32,
    pub confidence_score: f32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<GoldenRecordRow> for GoldenRecord {
    fn from(value: GoldenRecordRow) -> Self {
        Self {
            id: value.id,
            cluster_id: value.cluster_id,
            title: value.title,
            canonical_values: value.canonical_values.0,
            provenance: value.provenance.0,
            completeness_score: value.completeness_score,
            confidence_score: value.confidence_score,
            status: value.status,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
