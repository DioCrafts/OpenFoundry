use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    AppState,
    domain::{executor, lineage},
    models::workflow::{
        CreateWorkflowRequest, ListWorkflowsQuery, UpdateWorkflowRequest, WorkflowDefinition,
    },
};

pub async fn list_workflows(
    State(state): State<AppState>,
    Query(params): Query<ListWorkflowsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search_pattern = params.search.map(|value| format!("%{value}%"));

    let workflows = sqlx::query_as::<_, WorkflowDefinition>(
        r#"SELECT * FROM workflows
		   WHERE ($1::TEXT IS NULL OR name ILIKE $1 OR description ILIKE $1)
			 AND ($2::TEXT IS NULL OR trigger_type = $2)
			 AND ($3::TEXT IS NULL OR status = $3)
		   ORDER BY updated_at DESC
		   LIMIT $4 OFFSET $5"#,
    )
    .bind(&search_pattern)
    .bind(&params.trigger_type)
    .bind(&params.status)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM workflows
		   WHERE ($1::TEXT IS NULL OR name ILIKE $1 OR description ILIKE $1)
			 AND ($2::TEXT IS NULL OR trigger_type = $2)
			 AND ($3::TEXT IS NULL OR status = $3)"#,
    )
    .bind(&search_pattern)
    .bind(&params.trigger_type)
    .bind(&params.status)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    match workflows {
        Ok(data) => Json(serde_json::json!({
            "data": data,
            "page": page,
            "per_page": per_page,
            "total": total,
            "total_pages": (total as f64 / per_page as f64).ceil() as i64,
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("list workflows failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_workflow(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<CreateWorkflowRequest>,
) -> impl IntoResponse {
    let steps = match serde_json::to_value(&body.steps) {
        Ok(value) => value,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    let workflow = sqlx::query_as::<_, WorkflowDefinition>(
		r#"INSERT INTO workflows (
			   id, name, description, owner_id, status, trigger_type, trigger_config, steps, webhook_secret, next_run_at
		   )
		   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		   RETURNING *"#,
	)
	.bind(Uuid::now_v7())
	.bind(&body.name)
	.bind(body.description.as_deref().unwrap_or(""))
	.bind(claims.sub)
	.bind(body.status.as_deref().unwrap_or("draft"))
	.bind(&body.trigger_type)
	.bind(&body.trigger_config)
	.bind(&steps)
	.bind(
		body.trigger_config
			.get("secret")
			.and_then(Value::as_str)
			.map(str::to_string),
	)
	.bind(Option::<chrono::DateTime<chrono::Utc>>::None)
	.fetch_one(&state.db)
	.await;

    match workflow {
        Ok(mut workflow) => {
            let next_run_at = executor::compute_next_run_at(&workflow);
            if next_run_at.is_some() {
                if let Ok(updated) = sqlx::query_as::<_, WorkflowDefinition>(
                    r#"UPDATE workflows SET next_run_at = $2 WHERE id = $1 RETURNING *"#,
                )
                .bind(workflow.id)
                .bind(next_run_at)
                .fetch_one(&state.db)
                .await
                {
                    workflow = updated;
                }
            }

            if let Err(error) = lineage::sync_workflow_lineage(&state, &workflow).await {
                tracing::warn!(workflow_id = %workflow.id, "workflow lineage sync failed: {error}");
            }

            (StatusCode::CREATED, Json(workflow)).into_response()
        }
        Err(error) => {
            tracing::error!("create workflow failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    match load_workflow(&state, workflow_id).await {
        Ok(Some(workflow)) => Json(workflow).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get workflow failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Json(body): Json<UpdateWorkflowRequest>,
) -> impl IntoResponse {
    let Some(existing) = (match load_workflow(&state, workflow_id).await {
        Ok(workflow) => workflow,
        Err(error) => {
            tracing::error!("load workflow for update failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let next_name = body.name.unwrap_or(existing.name.clone());
    let next_description = body.description.unwrap_or(existing.description.clone());
    let next_status = body.status.unwrap_or(existing.status.clone());
    let next_trigger_type = body.trigger_type.unwrap_or(existing.trigger_type.clone());
    let next_trigger_config = body
        .trigger_config
        .unwrap_or(existing.trigger_config.clone());
    let next_steps = body
        .steps
        .map(|steps| serde_json::to_value(steps))
        .transpose()
        .map_err(|error| error.to_string());

    let Ok(next_steps) = next_steps else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid workflow steps" })),
        )
            .into_response();
    };

    let updated = sqlx::query_as::<_, WorkflowDefinition>(
        r#"UPDATE workflows
		   SET name = $2,
			   description = $3,
			   status = $4,
			   trigger_type = $5,
			   trigger_config = $6,
			   steps = $7,
			   webhook_secret = $8,
			   next_run_at = $9,
			   updated_at = NOW()
		   WHERE id = $1
		   RETURNING *"#,
    )
    .bind(workflow_id)
    .bind(&next_name)
    .bind(&next_description)
    .bind(&next_status)
    .bind(&next_trigger_type)
    .bind(&next_trigger_config)
    .bind(next_steps.unwrap_or(existing.steps.clone()))
    .bind(
        next_trigger_config
            .get("secret")
            .and_then(Value::as_str)
            .map(str::to_string),
    )
    .bind(Option::<chrono::DateTime<chrono::Utc>>::None)
    .fetch_one(&state.db)
    .await;

    match updated {
        Ok(mut workflow) => {
            let next_run_at = executor::compute_next_run_at(&workflow);
            if let Ok(reloaded) = sqlx::query_as::<_, WorkflowDefinition>(
                r#"UPDATE workflows SET next_run_at = $2 WHERE id = $1 RETURNING *"#,
            )
            .bind(workflow.id)
            .bind(next_run_at)
            .fetch_one(&state.db)
            .await
            {
                workflow = reloaded;
            }

            if let Err(error) = lineage::sync_workflow_lineage(&state, &workflow).await {
                tracing::warn!(workflow_id = %workflow.id, "workflow lineage sync failed: {error}");
            }

            Json(workflow).into_response()
        }
        Err(error) => {
            tracing::error!("update workflow failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM workflows WHERE id = $1")
        .bind(workflow_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            if let Err(error) = lineage::delete_workflow_lineage(&state, workflow_id).await {
                tracing::warn!(workflow_id = %workflow_id, "workflow lineage delete failed: {error}");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete workflow failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn load_workflow(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<Option<WorkflowDefinition>, sqlx::Error> {
    sqlx::query_as::<_, WorkflowDefinition>("SELECT * FROM workflows WHERE id = $1")
        .bind(workflow_id)
        .fetch_optional(&state.db)
        .await
}
