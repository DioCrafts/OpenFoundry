use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

use super::golden_record::GoldenRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    pub record_id: String,
    pub source: String,
    pub external_id: String,
    pub display_name: String,
    pub confidence: f32,
    pub attributes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvidence {
    pub left_record_id: String,
    pub right_record_id: String,
    pub blocking_key: String,
    pub rule_score: f32,
    pub ml_score: f32,
    pub final_score: f32,
    pub comparators: Vec<String>,
    pub explanation: String,
    pub requires_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewQueueItem {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub status: String,
    pub severity: String,
    pub recommended_action: String,
    pub rationale: Vec<String>,
    pub assigned_to: Option<String>,
    pub reviewed_by: Option<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCluster {
    pub id: Uuid,
    pub job_id: Uuid,
    pub cluster_key: String,
    pub status: String,
    pub records: Vec<EntityRecord>,
    pub evidence: Vec<MatchEvidence>,
    pub confidence_score: f32,
    pub requires_review: bool,
    pub suggested_golden_record_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterDetail {
    pub cluster: ResolvedCluster,
    pub review_item: Option<ReviewQueueItem>,
    pub golden_record: Option<GoldenRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitReviewRequest {
    pub decision: String,
    pub notes: Option<String>,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ClusterRow {
    pub id: Uuid,
    pub job_id: Uuid,
    pub cluster_key: String,
    pub status: String,
    pub records: SqlJson<Vec<EntityRecord>>,
    pub evidence: SqlJson<Vec<MatchEvidence>>,
    pub confidence_score: f32,
    pub requires_review: bool,
    pub suggested_golden_record_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReviewQueueRow {
    pub id: Uuid,
    pub cluster_id: Uuid,
    pub status: String,
    pub severity: String,
    pub recommended_action: String,
    pub rationale: SqlJson<Vec<String>>,
    pub assigned_to: Option<String>,
    pub reviewed_by: Option<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ClusterRow> for ResolvedCluster {
    fn from(value: ClusterRow) -> Self {
        Self {
            id: value.id,
            job_id: value.job_id,
            cluster_key: value.cluster_key,
            status: value.status,
            records: value.records.0,
            evidence: value.evidence.0,
            confidence_score: value.confidence_score,
            requires_review: value.requires_review,
            suggested_golden_record_id: value.suggested_golden_record_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<ReviewQueueRow> for ReviewQueueItem {
    fn from(value: ReviewQueueRow) -> Self {
        Self {
            id: value.id,
            cluster_id: value.cluster_id,
            status: value.status,
            severity: value.severity,
            recommended_action: value.recommended_action,
            rationale: value.rationale.0,
            assigned_to: value.assigned_to,
            reviewed_by: value.reviewed_by,
            notes: value.notes,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
