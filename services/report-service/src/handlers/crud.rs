use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::cron,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_all_reports,
        load_execution_history, load_report_row, not_found,
    },
    models::{
        ListResponse,
        report::{CreateReportRequest, ReportDefinition, UpdateReportRequest},
        snapshot::ReportOverview,
    },
};

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<ReportOverview> {
    let reports = load_all_reports(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let recent_executions = load_execution_history(&state.db, None, 6)
        .await
        .map_err(|cause| db_error(&cause))?;
    let active_schedules = reports
        .iter()
        .filter(|report| report.schedule.enabled)
        .count();
    let executions_24h = recent_executions
        .iter()
        .filter(|execution| execution.generated_at > Utc::now() - chrono::Duration::hours(24))
        .count();
    let mut generator_mix = reports
        .iter()
        .map(|report| report.generator_kind.label().to_string())
        .collect::<Vec<_>>();
    generator_mix.sort();
    generator_mix.dedup();

    Ok(Json(ReportOverview {
        report_count: reports.len(),
        active_schedules,
        executions_24h,
        generator_mix,
        latest_execution: recent_executions.first().cloned(),
    }))
}

pub async fn list_reports(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ReportDefinition>> {
    let reports = load_all_reports(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: reports }))
}

pub async fn create_report(
    State(state): State<AppState>,
    Json(request): Json<CreateReportRequest>,
) -> ServiceResult<ReportDefinition> {
    if request.name.trim().is_empty() {
        return Err(bad_request("report name is required"));
    }

    if request.template.sections.is_empty() {
        return Err(bad_request("at least one template section is required"));
    }

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let schedule = cron::normalize_schedule(request.schedule, now);
    let template = serde_json::to_value(&request.template)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let schedule_value =
        serde_json::to_value(&schedule).map_err(|cause| internal_error(cause.to_string()))?;
    let recipients = serde_json::to_value(&request.recipients)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let tags =
        serde_json::to_value(&request.tags).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO report_definitions (id, name, description, owner, generator_kind, dataset_name, template, schedule, recipients, tags, parameters, active, last_generated_at, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10::jsonb, $11::jsonb, $12, NULL, $13, $14)",
	)
	.bind(id)
	.bind(request.name)
	.bind(request.description)
	.bind(request.owner)
	.bind(request.generator_kind.as_str())
	.bind(request.dataset_name)
	.bind(template)
	.bind(schedule_value)
	.bind(recipients)
	.bind(tags)
	.bind(request.parameters)
	.bind(request.active)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_report_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created report could not be reloaded"))?;

    let report =
        ReportDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(report))
}

pub async fn update_report(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateReportRequest>,
) -> ServiceResult<ReportDefinition> {
    let existing_row = load_report_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("report not found"))?;
    let mut report = ReportDefinition::try_from(existing_row)
        .map_err(|cause| internal_error(cause.to_string()))?;

    if let Some(name) = request.name {
        if name.trim().is_empty() {
            return Err(bad_request("report name cannot be empty"));
        }
        report.name = name;
    }
    if let Some(description) = request.description {
        report.description = description;
    }
    if let Some(owner) = request.owner {
        report.owner = owner;
    }
    if let Some(generator_kind) = request.generator_kind {
        report.generator_kind = generator_kind;
    }
    if let Some(dataset_name) = request.dataset_name {
        report.dataset_name = dataset_name;
    }
    if let Some(template) = request.template {
        if template.sections.is_empty() {
            return Err(bad_request("template requires at least one section"));
        }
        report.template = template;
    }
    if let Some(schedule) = request.schedule {
        report.schedule = cron::normalize_schedule(schedule, Utc::now());
    }
    if let Some(recipients) = request.recipients {
        report.recipients = recipients;
    }
    if let Some(tags) = request.tags {
        report.tags = tags;
    }
    if let Some(parameters) = request.parameters {
        report.parameters = parameters;
    }
    if let Some(active) = request.active {
        report.active = active;
    }
    let now = Utc::now();
    let template = serde_json::to_value(&report.template)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let schedule_value = serde_json::to_value(&report.schedule)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let recipients = serde_json::to_value(&report.recipients)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let tags =
        serde_json::to_value(&report.tags).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        "UPDATE report_definitions
		 SET name = $2,
		     description = $3,
		     owner = $4,
		     generator_kind = $5,
		     dataset_name = $6,
		     template = $7::jsonb,
		     schedule = $8::jsonb,
		     recipients = $9::jsonb,
		     tags = $10::jsonb,
		     parameters = $11::jsonb,
		     active = $12,
		     updated_at = $13
		 WHERE id = $1",
    )
    .bind(id)
    .bind(&report.name)
    .bind(&report.description)
    .bind(&report.owner)
    .bind(report.generator_kind.as_str())
    .bind(&report.dataset_name)
    .bind(template)
    .bind(schedule_value)
    .bind(recipients)
    .bind(tags)
    .bind(&report.parameters)
    .bind(report.active)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_report_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated report could not be reloaded"))?;

    let report =
        ReportDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(report))
}
