use auth_middleware::{layer::AuthUser, tenant::TenantContext};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::AppState;
use crate::domain::executor::{datafusion, distributed};
use crate::models::saved_query::ExecuteQueryRequest;

/// POST /api/v1/queries/execute
pub async fn execute_query(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ExecuteQueryRequest>,
) -> impl IntoResponse {
    let tenant = TenantContext::from_claims(&claims);
    let limit = tenant.clamp_query_limit(body.limit.unwrap_or(1000).min(10_000));
    let execution_mode = body.execution_mode.as_deref().unwrap_or("auto");
    let configured_workers = tenant.clamp_query_workers(
        body.distributed_worker_count
            .unwrap_or_else(|| state.distributed_query_workers.max(1)),
    );

    let result = match execution_mode {
        "distributed" => {
            distributed::execute_distributed_query(
                state.query_ctx.clone(),
                &body.sql,
                limit,
                configured_workers.max(2),
            )
            .await
        }
        "local" => datafusion::execute_query(state.query_ctx.as_ref(), &body.sql, limit).await,
        "auto" => {
            if configured_workers > 1 {
                distributed::execute_distributed_query(
                    state.query_ctx.clone(),
                    &body.sql,
                    limit,
                    configured_workers,
                )
                .await
            } else {
                datafusion::execute_query(state.query_ctx.as_ref(), &body.sql, limit).await
            }
        }
        _ => Err(format!(
            "invalid execution_mode '{}'; expected 'auto', 'local', or 'distributed'",
            execution_mode,
        )),
    };

    match result {
        Ok(mut result) => {
            if let Some(execution) = result.execution.as_mut() {
                execution.mode = format!("{}:{}", execution.mode, tenant.tier);
            }
            Json(result).into_response()
        }
        Err(e) => {
            tracing::warn!(sql = %body.sql, error = %e, "query execution failed");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": e,
                })),
            )
                .into_response()
        }
    }
}
