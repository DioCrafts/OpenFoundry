use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::domain::lineage;
use auth_middleware::layer::AuthUser;

pub async fn get_dataset_lineage(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    match lineage::get_lineage_graph(&state.db, dataset_id).await {
        Ok(graph) => Json(graph).into_response(),
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

pub async fn get_full_lineage(_user: AuthUser, State(state): State<AppState>) -> impl IntoResponse {
    match lineage::get_full_lineage_graph(&state.db).await {
        Ok(graph) => Json(graph).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
