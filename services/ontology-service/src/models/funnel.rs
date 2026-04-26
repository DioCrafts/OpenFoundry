use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

fn default_preview_limit() -> i32 {
    500
}

fn default_funnel_status() -> String {
    "active".to_string()
}

fn default_marking() -> String {
    "public".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OntologyFunnelPropertyMapping {
    pub source_field: String,
    pub target_property: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct OntologyFunnelSourceRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub dataset_branch: Option<String>,
    pub dataset_version: Option<i32>,
    pub preview_limit: i32,
    pub default_marking: String,
    pub status: String,
    pub property_mappings: Value,
    pub trigger_context: Value,
    pub owner_id: Uuid,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyFunnelSource {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub dataset_branch: Option<String>,
    pub dataset_version: Option<i32>,
    pub preview_limit: i32,
    pub default_marking: String,
    pub status: String,
    pub property_mappings: Vec<OntologyFunnelPropertyMapping>,
    pub trigger_context: Value,
    pub owner_id: Uuid,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<OntologyFunnelSourceRow> for OntologyFunnelSource {
    type Error = serde_json::Error;

    fn try_from(row: OntologyFunnelSourceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            object_type_id: row.object_type_id,
            dataset_id: row.dataset_id,
            pipeline_id: row.pipeline_id,
            dataset_branch: row.dataset_branch,
            dataset_version: row.dataset_version,
            preview_limit: row.preview_limit,
            default_marking: row.default_marking,
            status: row.status,
            property_mappings: serde_json::from_value(row.property_mappings).unwrap_or_default(),
            trigger_context: row.trigger_context,
            owner_id: row.owner_id,
            last_run_at: row.last_run_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OntologyFunnelRun {
    pub id: Uuid,
    pub source_id: Uuid,
    pub object_type_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub pipeline_run_id: Option<Uuid>,
    pub status: String,
    pub trigger_type: String,
    pub started_by: Option<Uuid>,
    pub rows_read: i32,
    pub inserted_count: i32,
    pub updated_count: i32,
    pub skipped_count: i32,
    pub error_count: i32,
    pub details: Value,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOntologyFunnelSourceRequest {
    pub name: String,
    pub description: Option<String>,
    pub object_type_id: Uuid,
    pub dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub dataset_branch: Option<String>,
    pub dataset_version: Option<i32>,
    pub preview_limit: Option<i32>,
    pub default_marking: Option<String>,
    pub status: Option<String>,
    pub property_mappings: Option<Vec<OntologyFunnelPropertyMapping>>,
    pub trigger_context: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOntologyFunnelSourceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub pipeline_id: Option<Option<Uuid>>,
    pub dataset_branch: Option<Option<String>>,
    pub dataset_version: Option<Option<i32>>,
    pub preview_limit: Option<i32>,
    pub default_marking: Option<String>,
    pub status: Option<String>,
    pub property_mappings: Option<Vec<OntologyFunnelPropertyMapping>>,
    pub trigger_context: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListOntologyFunnelSourcesQuery {
    pub object_type_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListOntologyFunnelSourcesResponse {
    pub data: Vec<OntologyFunnelSource>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct TriggerOntologyFunnelRunRequest {
    pub limit: Option<i32>,
    pub dataset_branch: Option<String>,
    pub dataset_version: Option<i32>,
    #[serde(default)]
    pub skip_pipeline: bool,
    #[serde(default)]
    pub dry_run: bool,
    pub trigger_context: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListOntologyFunnelRunsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListOntologyFunnelRunsResponse {
    pub data: Vec<OntologyFunnelRun>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListOntologyFunnelHealthQuery {
    pub object_type_id: Option<Uuid>,
    pub stale_after_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GetOntologyFunnelSourceHealthQuery {
    pub stale_after_hours: Option<i64>,
}

#[derive(Debug, FromRow)]
pub struct OntologyFunnelHealthMetricsRow {
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub warning_runs: i64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
    pub latest_run_status: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_warning_at: Option<DateTime<Utc>>,
    pub rows_read: i64,
    pub inserted_count: i64,
    pub updated_count: i64,
    pub skipped_count: i64,
    pub error_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OntologyFunnelSourceHealth {
    pub source: OntologyFunnelSource,
    pub health_status: String,
    pub health_reason: String,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub warning_runs: i64,
    pub success_rate: f64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
    pub latest_run_status: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_warning_at: Option<DateTime<Utc>>,
    pub rows_read: i64,
    pub inserted_count: i64,
    pub updated_count: i64,
    pub skipped_count: i64,
    pub error_count: i64,
}

#[derive(Debug, Serialize)]
pub struct OntologyFunnelHealthResponse {
    pub stale_after_hours: i64,
    pub total_sources: i64,
    pub active_sources: i64,
    pub paused_sources: i64,
    pub healthy_sources: i64,
    pub degraded_sources: i64,
    pub failing_sources: i64,
    pub stale_sources: i64,
    pub never_run_sources: i64,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub warning_runs: i64,
    pub success_rate: f64,
    pub rows_read: i64,
    pub inserted_count: i64,
    pub updated_count: i64,
    pub skipped_count: i64,
    pub error_count: i64,
    pub last_run_at: Option<DateTime<Utc>>,
    pub sources: Vec<OntologyFunnelSourceHealth>,
}

#[derive(Debug, Serialize)]
pub struct OntologyFunnelSourceHealthResponse {
    pub stale_after_hours: i64,
    pub source_health: OntologyFunnelSourceHealth,
}

pub fn normalize_preview_limit(value: Option<i32>) -> i32 {
    value.unwrap_or_else(default_preview_limit).clamp(1, 1_000)
}

pub fn normalize_funnel_status(value: Option<String>) -> String {
    value.unwrap_or_else(default_funnel_status)
}

pub fn normalize_default_marking(value: Option<String>) -> String {
    value.unwrap_or_else(default_marking)
}

pub fn normalize_stale_after_hours(value: Option<i64>) -> i64 {
    value.unwrap_or(24).clamp(1, 24 * 30)
}
