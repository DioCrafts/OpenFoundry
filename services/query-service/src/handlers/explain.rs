use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppState;
use crate::domain::executor::datafusion;

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    pub sql: String,
}

/// POST /api/v1/queries/explain
pub async fn explain_query(
    State(state): State<AppState>,
    Json(body): Json<ExplainRequest>,
) -> impl IntoResponse {
    match datafusion::explain_query(&state.query_ctx, &body.sql).await {
        Ok((logical, physical)) => Json(serde_json::json!({
            "logical_plan": logical,
            "physical_plan": physical,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e,
            })),
        )
            .into_response(),
    }
}
