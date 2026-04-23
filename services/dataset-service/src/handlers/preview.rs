use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::models::dataset::Dataset;
use crate::models::schema::DatasetSchema;

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/v1/datasets/:id/preview
pub async fn preview_data(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(params): Query<PreviewQuery>,
) -> impl IntoResponse {
    let _limit = params.limit.unwrap_or(50).clamp(1, 1000);
    let _offset = params.offset.unwrap_or(0);

    let ds = match sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("preview lookup failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Try to read the data from storage — for MVP we return metadata only
    // Full Parquet reading comes in a later iteration
    let storage_path = format!("{}/v{}", ds.storage_path, ds.current_version);
    let exists = state.storage.exists(&storage_path).await.unwrap_or(false);

    if !exists {
        return Json(serde_json::json!({
            "dataset_id": dataset_id,
            "rows": [],
            "columns": [],
            "total_rows": 0,
            "message": "no data uploaded yet"
        })).into_response();
    }

    // Return basic info — full Parquet read is next milestone
    Json(serde_json::json!({
        "dataset_id": dataset_id,
        "version": ds.current_version,
        "size_bytes": ds.size_bytes,
        "format": ds.format,
        "message": "preview available after Parquet integration"
    })).into_response()
}

/// GET /api/v1/datasets/:id/schema
pub async fn get_schema(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let schema = sqlx::query_as::<_, DatasetSchema>(
        "SELECT * FROM dataset_schemas WHERE dataset_id = $1",
    )
    .bind(dataset_id)
    .fetch_optional(&state.db)
    .await;

    match schema {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "no schema found" }))).into_response(),
        Err(e) => {
            tracing::error!("get schema failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
