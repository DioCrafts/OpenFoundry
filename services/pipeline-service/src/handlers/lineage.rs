use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;
use crate::domain::lineage;
use auth_middleware::layer::AuthUser;

pub async fn get_dataset_lineage(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(error) = lineage::ensure_dataset_snapshot(&state, dataset_id).await {
        tracing::warn!(dataset_id = %dataset_id, "dataset snapshot refresh failed: {error}");
    }

    match lineage::get_lineage_graph(&state.db, dataset_id).await {
        Ok(graph) => Json(lineage::filter_graph_for_claims(graph, &claims)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_dataset_column_lineage(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    match lineage::get_dataset_column_lineage(&state.db, dataset_id).await {
        Ok(edges) => Json(edges).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_full_lineage(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match lineage::get_full_lineage_graph(&state.db).await {
        Ok(graph) => Json(lineage::filter_graph_for_claims(graph, &claims)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_dataset_lineage_impact(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(error) = lineage::ensure_dataset_snapshot(&state, dataset_id).await {
        tracing::warn!(dataset_id = %dataset_id, "dataset snapshot refresh failed: {error}");
    }

    match lineage::get_lineage_impact_analysis(&state.db, dataset_id).await {
        Ok(Some(impact)) => match lineage::filter_impact_for_claims(impact, &claims) {
            Ok(impact) => Json(impact).into_response(),
            Err(error) => (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response(),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

pub async fn trigger_dataset_lineage_builds(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(body): Json<lineage::LineageBuildRequest>,
) -> impl IntoResponse {
    if let Err(error) = lineage::ensure_dataset_snapshot(&state, dataset_id).await {
        tracing::warn!(dataset_id = %dataset_id, "dataset snapshot refresh failed: {error}");
    }

    match lineage::get_lineage_impact_analysis(&state.db, dataset_id).await {
        Ok(Some(impact)) => {
            if let Err(error) = lineage::filter_impact_for_claims(impact, &claims) {
                return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
            }
        }
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    }

    match lineage::trigger_lineage_builds(&state, dataset_id, claims.sub, body).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}

pub async fn sync_workflow_lineage(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Json(body): Json<lineage::WorkflowLineageSyncRequest>,
) -> impl IntoResponse {
    match lineage::sync_workflow_lineage(&state, workflow_id, body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}

pub async fn delete_workflow_lineage(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    match lineage::delete_workflow_lineage(&state.db, workflow_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}
