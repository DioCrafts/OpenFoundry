use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::function_package::FunctionPackageSummary;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FunctionPackageRun {
    pub id: Uuid,
    pub function_package_id: Uuid,
    pub function_package_name: String,
    pub function_package_version: String,
    pub runtime: String,
    pub status: String,
    pub invocation_kind: String,
    pub action_id: Option<Uuid>,
    pub action_name: Option<String>,
    pub object_type_id: Option<Uuid>,
    pub target_object_id: Option<Uuid>,
    pub actor_id: Uuid,
    pub duration_ms: i64,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListFunctionPackageRunsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub invocation_kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListFunctionPackageRunsResponse {
    pub data: Vec<FunctionPackageRun>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, FromRow)]
pub struct FunctionPackageMetricsRow {
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub simulation_runs: i64,
    pub action_runs: i64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct FunctionPackageMetricsResponse {
    pub package: FunctionPackageSummary,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub simulation_runs: i64,
    pub action_runs: i64,
    pub success_rate: f64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
}
