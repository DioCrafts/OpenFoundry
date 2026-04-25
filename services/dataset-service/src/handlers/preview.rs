use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, domain::runtime, models::schema::DatasetSchema};

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub version: Option<i32>,
    pub branch: Option<String>,
}

/// GET /api/v1/datasets/:id/preview
pub async fn preview_data(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(params): Query<PreviewQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 1_000);
    let offset = params.offset.unwrap_or(0).max(0);

    let source = match runtime::resolve_dataset_source(
        &state,
        dataset_id,
        params.branch.as_deref(),
        params.version,
    )
    .await
    {
        Ok(Some(source)) => source,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(runtime::DatasetSourceError::Invalid(message)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response();
        }
        Err(runtime::DatasetSourceError::Database(error)) => {
            tracing::error!("preview lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    match state.storage.get(&source.storage_path).await {
        Ok(bytes) => match runtime::prepare_query_context(&source.dataset.format, &bytes).await {
            Ok(prepared) => {
                let schema = runtime::load_schema_fields(&prepared.ctx).await;
                let total_rows = runtime::fetch_scalar_i64(
                    &prepared.ctx,
                    "SELECT COUNT(*) AS value FROM dataset",
                )
                .await;
                let rows = runtime::collect_object_rows(
                    &prepared.ctx,
                    &format!("SELECT * FROM dataset LIMIT {limit} OFFSET {offset}"),
                )
                .await;

                let columns = match schema {
                    Ok(columns) => columns,
                    Err(error) => {
                        errors.push(format!("schema inference failed: {error}"));
                        Vec::new()
                    }
                };
                let total_rows = match total_rows {
                    Ok(total_rows) => total_rows,
                    Err(error) => {
                        errors.push(format!("row counting failed: {error}"));
                        0
                    }
                };
                let rows = match rows {
                    Ok(rows) => rows,
                    Err(error) => {
                        errors.push(format!("row sampling failed: {error}"));
                        Vec::new()
                    }
                };

                if rows.is_empty() && total_rows > 0 {
                    warnings.push("requested page returned no rows".to_string());
                }

                runtime::cleanup_temp_path(prepared.path).await;

                return Json(runtime::preview_payload(
                    dataset_id,
                    source.branch,
                    source.version,
                    &source.dataset.format,
                    source.size_bytes,
                    source.storage_path,
                    limit,
                    offset,
                    total_rows,
                    columns,
                    rows,
                    warnings,
                    errors,
                ))
                .into_response();
            }
            Err(error) => {
                errors.push(format!("preview preparation failed: {error}"));
            }
        },
        Err(error) => {
            warnings.push("dataset has metadata but no readable storage object yet".to_string());
            errors.push(format!("storage read failed: {error}"));
        }
    }

    Json(runtime::preview_payload(
        dataset_id,
        source.branch,
        source.version,
        &source.dataset.format,
        source.size_bytes,
        source.storage_path,
        limit,
        offset,
        0,
        Vec::new(),
        Vec::new(),
        warnings,
        errors,
    ))
    .into_response()
}

/// GET /api/v1/datasets/:id/schema
pub async fn get_schema(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let schema =
        sqlx::query_as::<_, DatasetSchema>("SELECT * FROM dataset_schemas WHERE dataset_id = $1")
            .bind(dataset_id)
            .fetch_optional(&state.db)
            .await;

    match schema {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => match derive_schema(&state, dataset_id).await {
            Ok(Some(schema)) => Json(schema).into_response(),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "no schema found" })),
            )
                .into_response(),
            Err(error) => {
                tracing::error!("derive schema failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(error) => {
            tracing::error!("get schema failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn derive_schema(
    state: &AppState,
    dataset_id: Uuid,
) -> Result<Option<DatasetSchema>, String> {
    let source = match runtime::resolve_dataset_source(state, dataset_id, None, None)
        .await
        .map_err(|error| match error {
            runtime::DatasetSourceError::Invalid(message)
            | runtime::DatasetSourceError::Database(message) => message,
        })? {
        Some(source) => source,
        None => return Ok(None),
    };
    let bytes = match state.storage.get(&source.storage_path).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let prepared = runtime::prepare_query_context(&source.dataset.format, &bytes).await?;
    let fields = runtime::load_schema_fields(&prepared.ctx).await?;
    runtime::cleanup_temp_path(prepared.path).await;

    Ok(Some(DatasetSchema {
        id: Uuid::now_v7(),
        dataset_id,
        fields: runtime::schema_to_value(&fields)?,
        created_at: chrono::Utc::now(),
    }))
}
