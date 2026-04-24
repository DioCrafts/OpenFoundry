pub mod crud;
pub mod download;
pub mod generate;
pub mod schedule;

use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::models::{report::ReportRow, snapshot::ReportExecutionRow};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("report-service database error: {cause}");
    internal_error("database operation failed")
}

pub async fn load_report_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<ReportRow>, sqlx::Error> {
    sqlx::query_as::<_, ReportRow>(
		"SELECT id, name, description, owner, generator_kind, dataset_name, template, schedule, recipients, tags, parameters, active, last_generated_at, created_at, updated_at
		 FROM report_definitions
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_all_reports(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::report::ReportDefinition>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ReportRow>(
		"SELECT id, name, description, owner, generator_kind, dataset_name, template, schedule, recipients, tags, parameters, active, last_generated_at, created_at, updated_at
		 FROM report_definitions
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::report::ReportDefinition::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_execution_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<ReportExecutionRow>, sqlx::Error> {
    sqlx::query_as::<_, ReportExecutionRow>(
		"SELECT e.id, e.report_id, d.name AS report_name, e.status, e.generator_kind, e.triggered_by, e.generated_at, e.completed_at, e.preview, e.artifact, e.distributions, e.metrics
		 FROM report_executions e
		 JOIN report_definitions d ON d.id = e.report_id
		 WHERE e.id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_execution_history(
    db: &sqlx::PgPool,
    report_id: Option<uuid::Uuid>,
    limit: i64,
) -> Result<Vec<crate::models::snapshot::ReportExecution>, sqlx::Error> {
    let rows = if let Some(report_id) = report_id {
        sqlx::query_as::<_, ReportExecutionRow>(
			"SELECT e.id, e.report_id, d.name AS report_name, e.status, e.generator_kind, e.triggered_by, e.generated_at, e.completed_at, e.preview, e.artifact, e.distributions, e.metrics
			 FROM report_executions e
			 JOIN report_definitions d ON d.id = e.report_id
			 WHERE e.report_id = $1
			 ORDER BY e.generated_at DESC
			 LIMIT $2",
		)
		.bind(report_id)
		.bind(limit)
		.fetch_all(db)
		.await?
    } else {
        sqlx::query_as::<_, ReportExecutionRow>(
			"SELECT e.id, e.report_id, d.name AS report_name, e.status, e.generator_kind, e.triggered_by, e.generated_at, e.completed_at, e.preview, e.artifact, e.distributions, e.metrics
			 FROM report_executions e
			 JOIN report_definitions d ON d.id = e.report_id
			 ORDER BY e.generated_at DESC
			 LIMIT $1",
		)
		.bind(limit)
		.fetch_all(db)
		.await?
    };

    rows.into_iter()
        .map(crate::models::snapshot::ReportExecution::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}
