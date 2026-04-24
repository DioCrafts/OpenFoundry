use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::{AppState, domain::catalog};

/// GET /api/v1/datasets/catalog/facets
pub async fn get_catalog_facets(State(state): State<AppState>) -> impl IntoResponse {
    match catalog::fetch_catalog_facets(&state.db).await {
        Ok(facets) => Json(facets).into_response(),
        Err(error) => {
            tracing::error!("list catalog facets failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
