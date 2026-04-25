use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    domain::executor,
    handlers::crud::load_workflow,
    models::{
        execution::InternalLineageRunRequest, execution::StartRunRequest,
        execution::TriggerEventRequest, workflow::WorkflowDefinition,
    },
};

pub async fn start_manual_run(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<StartRunRequest>,
) -> impl IntoResponse {
    let Some(workflow) = (match load_workflow(&state, workflow_id).await {
        Ok(workflow) => workflow,
        Err(error) => {
            tracing::error!("manual run lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    match executor::execute_workflow_run(
        &state,
        &workflow,
        "manual",
        Some(claims.sub),
        body.context,
    )
    .await
    {
        Ok(run) => (StatusCode::CREATED, Json(run)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}

pub async fn trigger_event(
    State(state): State<AppState>,
    Path(event_name): Path<String>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<TriggerEventRequest>,
) -> impl IntoResponse {
    let workflows = sqlx::query_as::<_, WorkflowDefinition>(
        r#"SELECT * FROM workflows
		   WHERE status = 'active'
			 AND trigger_type = 'event'
			 AND trigger_config ->> 'event_name' = $1
		   ORDER BY updated_at DESC"#,
    )
    .bind(&event_name)
    .fetch_all(&state.db)
    .await;

    match workflows {
        Ok(workflows) => {
            let mut triggered = Vec::new();
            for workflow in workflows {
                let payload = json!({
                    "trigger": {
                        "type": "event",
                        "name": event_name,
                        "actor_id": claims.sub,
                    },
                    "event": body.context,
                });

                match executor::execute_workflow_run(
                    &state,
                    &workflow,
                    "event",
                    Some(claims.sub),
                    payload,
                )
                .await
                {
                    Ok(run) => triggered.push(run),
                    Err(error) => {
                        tracing::warn!(workflow_id = %workflow.id, "event trigger failed: {error}")
                    }
                }
            }

            Json(json!({ "data": triggered, "event_name": event_name })).into_response()
        }
        Err(error) => {
            tracing::error!("event workflow lookup failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn trigger_webhook(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<TriggerEventRequest>,
) -> impl IntoResponse {
    let Some(workflow) = (match load_workflow(&state, workflow_id).await {
        Ok(workflow) => workflow,
        Err(error) => {
            tracing::error!("webhook lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if workflow.trigger_type != "webhook" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "workflow is not configured for webhook triggers" })),
        )
            .into_response();
    }

    if let Some(expected_secret) = workflow.webhook_secret.as_deref() {
        let actual = headers
            .get("x-openfoundry-webhook-secret")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if actual != expected_secret {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    match executor::execute_workflow_run(
        &state,
        &workflow,
        "webhook",
        None,
        json!({
            "trigger": {
                "type": "webhook",
                "workflow_id": workflow_id,
            },
            "payload": body.context,
        }),
    )
    .await
    {
        Ok(run) => (StatusCode::CREATED, Json(run)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}

pub async fn run_due_cron_workflows(State(state): State<AppState>) -> impl IntoResponse {
    match executor::run_due_cron_workflows(&state).await {
        Ok(triggered_runs) => Json(json!({ "triggered_runs": triggered_runs })).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn start_internal_lineage_run(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Json(body): Json<InternalLineageRunRequest>,
) -> impl IntoResponse {
    let Some(workflow) = (match load_workflow(&state, workflow_id).await {
        Ok(workflow) => workflow,
        Err(error) => {
            tracing::error!("internal lineage run lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if workflow.status != "active" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "workflow must be active to run from lineage" })),
        )
            .into_response();
    }

    match executor::execute_workflow_run(&state, &workflow, "lineage_build", None, body.context)
        .await
    {
        Ok(run) => (StatusCode::CREATED, Json(run)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}
