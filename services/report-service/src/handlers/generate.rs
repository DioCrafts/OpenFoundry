use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::Utc;

use crate::{
    AppState,
    domain::{data_fetcher, distribution, generators},
    handlers::{
        ServiceResult, db_error, internal_error, load_execution_history, load_execution_row,
        load_report_row, not_found,
    },
    models::{
        ListResponse,
        report::ReportDefinition,
        snapshot::{ReportCatalog, ReportExecution},
    },
};

pub async fn get_catalog() -> ServiceResult<ReportCatalog> {
    Ok(Json(ReportCatalog {
        generators: generators::catalog(),
        delivery_channels: distribution::catalog(),
    }))
}

pub async fn generate_report(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ServiceResult<ReportExecution> {
    let report_row = load_report_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("report not found"))?;
    let report = ReportDefinition::try_from(report_row)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let generated_at = Utc::now();
    let execution_id = uuid::Uuid::now_v7();
    let snapshot = data_fetcher::build_snapshot(&state, &report, &headers).await;
    let generated = generators::generate(&report, &snapshot, execution_id, generated_at);
    let distributions = distribution::deliver_report(
        &state,
        &report,
        execution_id,
        generated_at,
        &generated.preview,
        &generated.artifact,
        &generated.metrics,
    )
    .await;
    let preview = serde_json::to_value(&generated.preview)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let artifact = serde_json::to_value(&generated.artifact)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let distribution_rows =
        serde_json::to_value(&distributions).map_err(|cause| internal_error(cause.to_string()))?;
    let metrics = serde_json::to_value(&generated.metrics)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO report_executions (id, report_id, status, generator_kind, triggered_by, generated_at, completed_at, preview, artifact, distributions, metrics)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10::jsonb, $11::jsonb)",
	)
	.bind(execution_id)
	.bind(report.id)
	.bind("completed")
	.bind(report.generator_kind.as_str())
	.bind("manual")
	.bind(generated_at)
	.bind(generated_at)
	.bind(preview)
	.bind(artifact)
	.bind(distribution_rows)
	.bind(metrics)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    sqlx::query(
        "UPDATE report_definitions SET last_generated_at = $2, updated_at = $3 WHERE id = $1",
    )
    .bind(report.id)
    .bind(generated_at)
    .bind(generated_at)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_execution_row(&state.db, execution_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("generated execution could not be reloaded"))?;
    let execution =
        ReportExecution::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(execution))
}

pub async fn get_execution(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ReportExecution> {
    let row = load_execution_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("report execution not found"))?;
    let execution =
        ReportExecution::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(execution))
}

pub async fn list_history(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ReportExecution>> {
    let history = load_execution_history(&state.db, Some(id), 12)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: history }))
}
