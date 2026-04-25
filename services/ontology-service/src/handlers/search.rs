use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{graph, search},
    models::{
        graph::{GraphQuery, GraphResponse},
        search::{SearchRequest, SearchResponse},
    },
};

pub async fn search_ontology(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SearchRequest>,
) -> impl IntoResponse {
    if body.query.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "query is required" })),
        )
            .into_response();
    }

    match search::search_ontology(&state, &claims, &body).await {
        Ok(results) => Json(SearchResponse {
            query: body.query,
            total: results.len(),
            data: results,
        })
        .into_response(),
        Err(error) => {
            tracing::error!("ontology search failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_graph(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<GraphQuery>,
) -> impl IntoResponse {
    match graph::build_graph(&state, &claims, &query).await {
        Ok(graph) => Json::<GraphResponse>(graph).into_response(),
        Err(error) if error.contains("forbidden") => {
            (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response()
        }
        Err(error) if error.contains("not found") => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": error }))).into_response()
        }
        Err(error) => {
            tracing::error!("ontology graph failed: {error}");
            (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
        }
    }
}
