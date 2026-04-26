use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, models::function_package::FunctionPackageSummary};

pub struct FunctionPackageRunContext<'a> {
    pub invocation_kind: &'a str,
    pub action_id: Option<Uuid>,
    pub action_name: Option<&'a str>,
    pub object_type_id: Option<Uuid>,
    pub target_object_id: Option<Uuid>,
    pub actor_id: Uuid,
}

pub async fn record_function_package_run(
    state: &AppState,
    package: &FunctionPackageSummary,
    context: &FunctionPackageRunContext<'_>,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    duration_ms: i64,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    sqlx::query(
        r#"INSERT INTO ontology_function_package_runs (
               id,
               function_package_id,
               function_package_name,
               function_package_version,
               runtime,
               status,
               invocation_kind,
               action_id,
               action_name,
               object_type_id,
               target_object_id,
               actor_id,
               duration_ms,
               error_message,
               started_at,
               completed_at
           )
           VALUES (
               $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
           )"#,
    )
    .bind(Uuid::now_v7())
    .bind(package.id)
    .bind(&package.name)
    .bind(&package.version)
    .bind(&package.runtime)
    .bind(status)
    .bind(context.invocation_kind)
    .bind(context.action_id)
    .bind(context.action_name)
    .bind(context.object_type_id)
    .bind(context.target_object_id)
    .bind(context.actor_id)
    .bind(duration_ms.max(0))
    .bind(error_message)
    .bind(started_at)
    .bind(completed_at)
    .execute(&state.db)
    .await
    .map_err(|error| format!("failed to record function package run: {error}"))?;

    Ok(())
}
