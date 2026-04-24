use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, models::dataset::Dataset};

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub prefix: Option<String>,
}

pub async fn list_files(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(query): Query<FilesQuery>,
) -> impl IntoResponse {
    let dataset = match sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("dataset files lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let prefix = query
        .prefix
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("{}/{}", dataset.storage_path, value.trim_start_matches('/')))
        .unwrap_or_else(|| dataset.storage_path.clone());

    match state.storage.list(&prefix).await {
        Ok(files) => Json(serde_json::json!({
            "dataset_id": dataset_id,
            "root": dataset.storage_path,
            "current_version": dataset.current_version,
            "items": files.into_iter().map(|file| {
                serde_json::json!({
                    "path": file.path,
                    "size": file.size,
                    "last_modified": file.last_modified,
                    "content_type": file.content_type,
                })
            }).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("dataset files listing failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
