use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    models::execution::{ListRunsQuery, WorkflowRun},
};

pub async fn list_runs(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Query(params): Query<ListRunsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let runs = sqlx::query_as::<_, WorkflowRun>(
        r#"SELECT * FROM workflow_runs
		   WHERE workflow_id = $1
		   ORDER BY started_at DESC
		   LIMIT $2 OFFSET $3"#,
    )
    .bind(workflow_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM workflow_runs WHERE workflow_id = $1"#,
    )
    .bind(workflow_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    match runs {
        Ok(data) => Json(serde_json::json!({
            "data": data,
            "page": page,
            "per_page": per_page,
            "total": total,
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("list workflow runs failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
