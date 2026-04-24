use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::BytesMut;
use uuid::Uuid;

use crate::AppState;
use crate::domain::quality::profiler;
use crate::models::dataset::Dataset;

/// POST /api/v1/datasets/:id/upload
pub async fn upload_data(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Verify dataset exists
    let ds = match sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("upload lookup failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Read file from multipart
    let mut file_data = BytesMut::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(data) => file_data.extend_from_slice(&data),
                Err(e) => {
                    tracing::error!("failed to read upload: {e}");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": "failed to read file" })),
                    )
                        .into_response();
                }
            }
        }
    }

    if file_data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "no file provided" })),
        )
            .into_response();
    }

    let data = file_data.freeze();
    let size = data.len() as i64;
    let latest_version = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(version) FROM dataset_versions WHERE dataset_id = $1",
    )
    .bind(dataset_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(None)
    .unwrap_or(ds.current_version.saturating_sub(1));
    let new_version = latest_version + 1;
    let version_path = format!("{}/v{new_version}", ds.storage_path);

    // Upload to storage
    if let Err(e) = state.storage.put(&version_path, data.clone()).await {
        tracing::error!("storage upload failed: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "storage upload failed" })),
        )
            .into_response();
    }

    // Create version record
    let version_id = Uuid::now_v7();
    let _ = sqlx::query(
        r#"INSERT INTO dataset_versions (id, dataset_id, version, size_bytes, storage_path)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(version_id)
    .bind(dataset_id)
    .bind(new_version)
    .bind(size)
    .bind(&version_path)
    .execute(&state.db)
    .await;

    // Update dataset
    let _ = sqlx::query(
        "UPDATE datasets SET current_version = $2, size_bytes = $3, updated_at = NOW() WHERE id = $1",
    )
    .bind(dataset_id)
    .bind(new_version)
    .bind(size)
    .execute(&state.db)
    .await;

    let _ = sqlx::query(
        r#"UPDATE dataset_branches
           SET version = $3,
               updated_at = NOW()
           WHERE dataset_id = $1 AND name = $2"#,
    )
    .bind(dataset_id)
    .bind(&ds.active_branch)
    .bind(new_version)
    .execute(&state.db)
    .await;

    let refreshed_dataset = sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

    if let Some(dataset) = refreshed_dataset {
        if let Err(error) =
            profiler::refresh_dataset_quality(&state, &dataset, Some(data.clone())).await
        {
            tracing::warn!(dataset_id = %dataset_id, "quality refresh failed after upload: {error}");
        }
    }

    tracing::info!(dataset_id = %dataset_id, version = new_version, "data uploaded");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "dataset_id": dataset_id,
            "version": new_version,
            "size_bytes": size,
        })),
    )
        .into_response()
}
