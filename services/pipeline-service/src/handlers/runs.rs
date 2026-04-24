use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::run::*;
use auth_middleware::layer::AuthUser;

pub async fn list_runs(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(pipeline_id): Path<Uuid>,
    Query(params): Query<ListRunsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let runs = sqlx::query_as::<_, PipelineRun>(
        r#"SELECT * FROM pipeline_runs
           WHERE pipeline_id = $1
           ORDER BY started_at DESC LIMIT $2 OFFSET $3"#,
    )
    .bind(pipeline_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "data": runs }))
}

pub async fn get_run(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_pipeline_id, run_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, PipelineRun>("SELECT * FROM pipeline_runs WHERE id = $1")
        .bind(run_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(r)) => Json(serde_json::json!(r)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
