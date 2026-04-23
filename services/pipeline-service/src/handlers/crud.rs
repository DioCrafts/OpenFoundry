use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::domain::executor;
use crate::models::pipeline::*;
use crate::AppState;
use auth_middleware::layer::AuthUser;

pub async fn create_pipeline(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreatePipelineRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let description = body.description.unwrap_or_default();
    let dag = serde_json::to_value(&body.nodes).unwrap_or_default();
    let status = body.status.unwrap_or_else(|| "draft".to_string());
    let schedule_config = body.schedule_config.unwrap_or_default();
    let retry_policy = body.retry_policy.unwrap_or_default();
    let next_run_at = executor::compute_next_run_at_from_parts(&status, &schedule_config);

    let result = sqlx::query_as::<_, Pipeline>(
        r#"INSERT INTO pipelines (
               id, name, description, owner_id, dag, status, schedule_config, retry_policy, next_run_at
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&description)
    .bind(claims.sub)
    .bind(&dag)
    .bind(&status)
    .bind(serde_json::to_value(&schedule_config).unwrap_or_else(|_| json!({})))
    .bind(serde_json::to_value(&retry_policy).unwrap_or_else(|_| json!({})))
    .bind(next_run_at)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(p) => (StatusCode::CREATED, Json(serde_json::json!(p))).into_response(),
        Err(e) => {
            tracing::error!("create pipeline: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_pipelines(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListPipelinesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let pattern = format!("%{search}%");

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM pipelines
           WHERE name ILIKE $1
             AND ($2::TEXT IS NULL OR status = $2)"#,
    )
    .bind(&pattern)
    .bind(&params.status)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pipelines = sqlx::query_as::<_, Pipeline>(
        r#"SELECT * FROM pipelines
           WHERE name ILIKE $1
             AND ($2::TEXT IS NULL OR status = $2)
           ORDER BY created_at DESC LIMIT $3 OFFSET $4"#,
    )
    .bind(&pattern)
    .bind(&params.status)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(ListPipelinesResponse { data: pipelines, total, page, per_page })
}

pub async fn get_pipeline(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(p)) => Json(serde_json::json!(p)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_pipeline(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePipelineRequest>,
) -> impl IntoResponse {
    let existing = match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(pipeline)) => pipeline,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    };

    let existing_name = existing.name.clone();
    let existing_description = existing.description.clone();
    let existing_status = existing.status.clone();
    let existing_dag = existing.dag.clone();
    let existing_schedule = existing.schedule();
    let existing_retry_policy = existing.parsed_retry_policy();

    let name = body.name.unwrap_or(existing_name);
    let description = body.description.unwrap_or(existing_description);
    let status = body.status.unwrap_or(existing_status);
    let dag = body
        .nodes
        .map(|nodes| serde_json::to_value(nodes).unwrap_or_default())
        .unwrap_or(existing_dag);
    let schedule_config = body.schedule_config.unwrap_or(existing_schedule);
    let retry_policy = body
        .retry_policy
        .unwrap_or(existing_retry_policy);
    let next_run_at = executor::compute_next_run_at_from_parts(&status, &schedule_config);

    let result = sqlx::query_as::<_, Pipeline>(
        r#"UPDATE pipelines SET
           name = $2,
           description = $3,
           dag = $4,
           status = $5,
           schedule_config = $6,
           retry_policy = $7,
           next_run_at = $8,
           updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&name)
    .bind(&description)
    .bind(&dag)
    .bind(&status)
    .bind(serde_json::to_value(&schedule_config).unwrap_or_else(|_| json!({})))
    .bind(serde_json::to_value(&retry_policy).unwrap_or_else(|_| json!({})))
    .bind(next_run_at)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(p)) => Json(serde_json::json!(p)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_pipeline(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM pipelines WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
