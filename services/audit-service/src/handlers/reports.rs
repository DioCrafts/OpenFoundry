use axum::{Json, extract::State};

use crate::{
    AppState,
    domain::{export, gdpr},
    handlers::{ServiceResult, db_error, internal_error, load_events, load_policies, load_reports},
    models::{
        ListResponse,
        compliance_report::{
            ComplianceReport, ComplianceReportRequest, GdprEraseRequest, GdprEraseResponse,
            GdprExportPayload, GdprExportRequest,
        },
    },
};

pub async fn list_reports(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ComplianceReport>> {
    let reports = load_reports(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: reports }))
}

pub async fn generate_report(
    State(state): State<AppState>,
    Json(request): Json<ComplianceReportRequest>,
) -> ServiceResult<ComplianceReport> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let policies = load_policies(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let report = export::build_report(&request, &events, &policies);
    let findings = serde_json::to_value(&report.findings)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let artifact = serde_json::to_value(&report.artifact)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO compliance_reports (id, standard, title, scope, window_start, window_end, generated_at, status, findings, artifact, relevant_event_count, policy_count, control_summary, expires_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, $11, $12, $13, $14)",
	)
	.bind(report.id)
	.bind(report.standard.as_str())
	.bind(&report.title)
	.bind(&report.scope)
	.bind(report.window_start)
	.bind(report.window_end)
	.bind(report.generated_at)
	.bind(&report.status)
	.bind(findings)
	.bind(artifact)
	.bind(report.relevant_event_count)
	.bind(report.policy_count)
	.bind(&report.control_summary)
	.bind(report.expires_at)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    Ok(Json(report))
}

pub async fn export_subject_data(
    State(state): State<AppState>,
    Json(request): Json<GdprExportRequest>,
) -> ServiceResult<GdprExportPayload> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(gdpr::export_payload(&request, &events)))
}

pub async fn erase_subject_data(
    State(state): State<AppState>,
    Json(request): Json<GdprEraseRequest>,
) -> ServiceResult<GdprEraseResponse> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let response = gdpr::erase_response(&request, &events);

    sqlx::query(
		"UPDATE audit_events
		 SET metadata = jsonb_set(metadata, '{masked}', 'true'::jsonb, true), subject_id = CASE WHEN $2 THEN subject_id ELSE NULL END
		 WHERE subject_id = $1",
	)
	.bind(&request.subject_id)
	.bind(request.legal_hold)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    Ok(Json(response))
}
