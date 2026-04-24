use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::version::DatasetVersion;

/// GET /api/v1/datasets/:id/versions
pub async fn list_versions(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let versions = sqlx::query_as::<_, DatasetVersion>(
        "SELECT * FROM dataset_versions WHERE dataset_id = $1 ORDER BY version DESC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await;

    match versions {
        Ok(v) => Json(v).into_response(),
        Err(e) => {
            tracing::error!("list versions failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
