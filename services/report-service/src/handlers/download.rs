use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    handlers::{ServiceResult, db_error, internal_error, load_execution_row, not_found},
    models::snapshot::{DownloadPayload, ReportExecution},
};

pub async fn download_execution(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<DownloadPayload> {
    let row = load_execution_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("report execution not found"))?;
    let execution =
        ReportExecution::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;

    Ok(Json(DownloadPayload {
        file_name: execution.artifact.file_name.clone(),
        mime_type: execution.artifact.mime_type.clone(),
        storage_url: execution.artifact.storage_url.clone(),
        preview_excerpt: execution.preview.headline.clone(),
        report_name: execution.report_name,
    }))
}
