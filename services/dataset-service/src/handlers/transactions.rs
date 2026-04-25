use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{AppState, models::transaction::DatasetTransaction};

pub async fn list_transactions(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, DatasetTransaction>(
        "SELECT * FROM dataset_transactions WHERE dataset_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(transactions) => Json(transactions).into_response(),
        Err(error) => {
            tracing::error!("list dataset transactions failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
