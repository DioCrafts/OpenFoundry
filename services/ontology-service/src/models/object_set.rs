use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json as SqlJson};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectSetPolicy {
    #[serde(default)]
    pub allowed_markings: Vec<String>,
    pub minimum_clearance: Option<String>,
    #[serde(default)]
    pub deny_guest_sessions: bool,
    pub required_restricted_view_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSetFilter {
    pub field: String,
    pub operator: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSetTraversal {
    pub direction: String,
    pub link_type_id: Option<Uuid>,
    pub target_object_type_id: Option<Uuid>,
    pub max_hops: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSetJoin {
    pub secondary_object_type_id: Uuid,
    pub left_field: String,
    pub right_field: String,
    pub join_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSetDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub base_object_type_id: Uuid,
    pub filters: Vec<ObjectSetFilter>,
    pub traversals: Vec<ObjectSetTraversal>,
    pub join: Option<ObjectSetJoin>,
    pub projections: Vec<String>,
    pub what_if_label: Option<String>,
    pub policy: ObjectSetPolicy,
    pub materialized_snapshot: Option<Value>,
    pub materialized_at: Option<DateTime<Utc>>,
    pub materialized_row_count: i32,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateObjectSetRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub base_object_type_id: Uuid,
    #[serde(default)]
    pub filters: Vec<ObjectSetFilter>,
    #[serde(default)]
    pub traversals: Vec<ObjectSetTraversal>,
    pub join: Option<ObjectSetJoin>,
    #[serde(default)]
    pub projections: Vec<String>,
    pub what_if_label: Option<String>,
    #[serde(default)]
    pub policy: ObjectSetPolicy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateObjectSetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub base_object_type_id: Option<Uuid>,
    pub filters: Option<Vec<ObjectSetFilter>>,
    pub traversals: Option<Vec<ObjectSetTraversal>>,
    pub join: Option<ObjectSetJoin>,
    pub projections: Option<Vec<String>>,
    pub what_if_label: Option<String>,
    pub policy: Option<ObjectSetPolicy>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EvaluateObjectSetRequest {
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectSetEvaluationResponse {
    pub object_set: ObjectSetDefinition,
    pub total_base_matches: usize,
    pub total_rows: usize,
    pub traversal_neighbor_count: usize,
    pub rows: Vec<Value>,
    pub generated_at: DateTime<Utc>,
    pub materialized: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListObjectSetsResponse {
    pub data: Vec<ObjectSetDefinition>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ObjectSetRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub base_object_type_id: Uuid,
    pub filters: SqlJson<Vec<ObjectSetFilter>>,
    pub traversals: SqlJson<Vec<ObjectSetTraversal>>,
    pub join_config: Option<SqlJson<ObjectSetJoin>>,
    pub projections: SqlJson<Vec<String>>,
    pub what_if_label: Option<String>,
    pub policy: SqlJson<ObjectSetPolicy>,
    pub materialized_snapshot: Option<Value>,
    pub materialized_at: Option<DateTime<Utc>>,
    pub materialized_row_count: i32,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ObjectSetRow> for ObjectSetDefinition {
    fn from(value: ObjectSetRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            base_object_type_id: value.base_object_type_id,
            filters: value.filters.0,
            traversals: value.traversals.0,
            join: value.join_config.map(|item| item.0),
            projections: value.projections.0,
            what_if_label: value.what_if_label,
            policy: value.policy.0,
            materialized_snapshot: value.materialized_snapshot,
            materialized_at: value.materialized_at,
            materialized_row_count: value.materialized_row_count,
            owner_id: value.owner_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
