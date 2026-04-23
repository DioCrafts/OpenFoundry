pub mod events;
pub mod policies;
pub mod reports;

use axum::{http::StatusCode, Json};
use serde::Serialize;

use crate::models::{
	audit_event::AuditEventRow,
	compliance_report::ComplianceReportRow,
	policy::PolicyRow,
};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
	pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
	(StatusCode::BAD_REQUEST, Json(ErrorResponse { error: message.into() }))
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
	(StatusCode::NOT_FOUND, Json(ErrorResponse { error: message.into() }))
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
	(StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: message.into() }))
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
	tracing::error!("audit-service database error: {cause}");
	internal_error("database operation failed")
}

pub async fn load_events(db: &sqlx::PgPool) -> Result<Vec<crate::models::audit_event::AuditEvent>, sqlx::Error> {
	let rows = sqlx::query_as::<_, AuditEventRow>(
		"SELECT id, sequence, previous_hash, entry_hash, source_service, channel, actor, action, resource_type, resource_id, status, severity, classification, subject_id, ip_address, location, metadata, labels, retention_until, occurred_at, ingested_at
		 FROM audit_events
		 ORDER BY sequence DESC",
	)
	.fetch_all(db)
	.await?;

	rows.into_iter()
		.map(crate::models::audit_event::AuditEvent::try_from)
		.collect::<Result<Vec<_>, _>>()
		.map_err(|cause| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, cause))))
}

pub async fn load_event_row(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<Option<AuditEventRow>, sqlx::Error> {
	sqlx::query_as::<_, AuditEventRow>(
		"SELECT id, sequence, previous_hash, entry_hash, source_service, channel, actor, action, resource_type, resource_id, status, severity, classification, subject_id, ip_address, location, metadata, labels, retention_until, occurred_at, ingested_at
		 FROM audit_events WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_policies(db: &sqlx::PgPool) -> Result<Vec<crate::models::policy::AuditPolicy>, sqlx::Error> {
	let rows = sqlx::query_as::<_, PolicyRow>(
		"SELECT id, name, description, scope, classification, retention_days, legal_hold, purge_mode, active, rules, updated_by, created_at, updated_at
		 FROM audit_policies
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

	rows.into_iter()
		.map(crate::models::policy::AuditPolicy::try_from)
		.collect::<Result<Vec<_>, _>>()
		.map_err(|cause| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, cause))))
}

pub async fn load_policy_row(db: &sqlx::PgPool, id: uuid::Uuid) -> Result<Option<PolicyRow>, sqlx::Error> {
	sqlx::query_as::<_, PolicyRow>(
		"SELECT id, name, description, scope, classification, retention_days, legal_hold, purge_mode, active, rules, updated_by, created_at, updated_at
		 FROM audit_policies WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_reports(db: &sqlx::PgPool) -> Result<Vec<crate::models::compliance_report::ComplianceReport>, sqlx::Error> {
	let rows = sqlx::query_as::<_, ComplianceReportRow>(
		"SELECT id, standard, title, scope, window_start, window_end, generated_at, status, findings, artifact, relevant_event_count, policy_count, control_summary, expires_at
		 FROM compliance_reports
		 ORDER BY generated_at DESC",
	)
	.fetch_all(db)
	.await?;

	rows.into_iter()
		.map(crate::models::compliance_report::ComplianceReport::try_from)
		.collect::<Result<Vec<_>, _>>()
		.map_err(|cause| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, cause))))
}