use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;
use crate::domain::executor;
use crate::models::pipeline::Pipeline;
use crate::models::run::{PipelineRun, RetryPipelineRunRequest, TriggerPipelineRequest};
use auth_middleware::{layer::AuthUser, tenant::TenantContext};

pub async fn trigger_run(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(pipeline_id): Path<Uuid>,
    Json(body): Json<TriggerPipelineRequest>,
) -> impl IntoResponse {
    let tenant = TenantContext::from_claims(&claims);
    let pipeline = match load_pipeline(&state, pipeline_id).await {
        Ok(Some(pipeline)) => pipeline,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };

    let mut context = body.context.unwrap_or_else(|| {
        json!({
            "trigger": {
                "type": "manual",
                "started_at": chrono::Utc::now(),
            }
        })
    });
    attach_tenant_context(&mut context, &tenant);

    match executor::start_pipeline_run(
        &state,
        &pipeline,
        Some(claims.sub),
        "manual",
        body.from_node_id,
        None,
        1,
        tenant.clamp_pipeline_workers(state.distributed_pipeline_workers),
        body.skip_unchanged,
        context,
    )
    .await
    {
        Ok(run) => (StatusCode::CREATED, Json(serde_json::json!(run))).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    }
}

pub async fn retry_run(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((pipeline_id, run_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RetryPipelineRunRequest>,
) -> impl IntoResponse {
    let tenant = TenantContext::from_claims(&claims);
    let pipeline = match load_pipeline(&state, pipeline_id).await {
        Ok(Some(pipeline)) => pipeline,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };

    let previous_run = match sqlx::query_as::<_, PipelineRun>(
        "SELECT * FROM pipeline_runs WHERE id = $1 AND pipeline_id = $2",
    )
    .bind(run_id)
    .bind(pipeline_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(run)) => run,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };

    match executor::retry_pipeline_run(
        &state,
        &pipeline,
        &previous_run,
        body.from_node_id,
        tenant.clamp_pipeline_workers(state.distributed_pipeline_workers),
        body.skip_unchanged,
    )
    .await
    {
        Ok(run) => (StatusCode::CREATED, Json(serde_json::json!(run))).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn run_due_scheduled_pipelines(
    _user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match executor::run_due_scheduled_pipelines(&state).await {
        Ok(triggered_runs) => Json(json!({ "triggered_runs": triggered_runs })).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    }
}

async fn load_pipeline(
    state: &AppState,
    pipeline_id: Uuid,
) -> Result<Option<Pipeline>, sqlx::Error> {
    sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id = $1")
        .bind(pipeline_id)
        .fetch_optional(&state.db)
        .await
}

fn attach_tenant_context(context: &mut serde_json::Value, tenant: &TenantContext) {
    let tenant_json = json!({
        "scope_id": tenant.scope_id,
        "tenant_id": tenant.tenant_id,
        "tier": tenant.tier,
        "workspace": tenant.workspace,
        "quotas": {
            "max_query_limit": tenant.quotas.max_query_limit,
            "max_distributed_query_workers": tenant.quotas.max_distributed_query_workers,
            "max_pipeline_workers": tenant.quotas.max_pipeline_workers,
            "max_request_body_bytes": tenant.quotas.max_request_body_bytes,
            "requests_per_minute": tenant.quotas.requests_per_minute,
        },
    });

    match context {
        serde_json::Value::Object(map) => {
            map.insert("tenant".to_string(), tenant_json);
        }
        other => {
            *other = json!({ "tenant": tenant_json, "context": other.clone() });
        }
    }
}
