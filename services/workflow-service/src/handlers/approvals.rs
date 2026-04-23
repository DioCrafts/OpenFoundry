use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::IntoResponse,
	Json,
};
use uuid::Uuid;

use crate::{
	domain::executor,
	models::{
		approval::{ApprovalDecisionRequest, ListApprovalsQuery, WorkflowApproval},
		execution::WorkflowRun,
		workflow::{WorkflowDefinition, WorkflowStep},
	},
	AppState,
};

pub async fn list_approvals(
	State(state): State<AppState>,
	Query(params): Query<ListApprovalsQuery>,
) -> impl IntoResponse {
	let page = params.page.unwrap_or(1).max(1);
	let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
	let offset = (page - 1) * per_page;

	let approvals = sqlx::query_as::<_, WorkflowApproval>(
		r#"SELECT * FROM workflow_approvals
		   WHERE ($1::TEXT IS NULL OR status = $1)
			 AND ($2::UUID IS NULL OR assigned_to = $2)
			 AND ($3::UUID IS NULL OR workflow_id = $3)
		   ORDER BY requested_at DESC
		   LIMIT $4 OFFSET $5"#,
	)
	.bind(&params.status)
	.bind(params.assigned_to)
	.bind(params.workflow_id)
	.bind(per_page)
	.bind(offset)
	.fetch_all(&state.db)
	.await;

	let total = sqlx::query_scalar::<_, i64>(
		r#"SELECT COUNT(*) FROM workflow_approvals
		   WHERE ($1::TEXT IS NULL OR status = $1)
			 AND ($2::UUID IS NULL OR assigned_to = $2)
			 AND ($3::UUID IS NULL OR workflow_id = $3)"#,
	)
	.bind(&params.status)
	.bind(params.assigned_to)
	.bind(params.workflow_id)
	.fetch_one(&state.db)
	.await
	.unwrap_or(0);

	match approvals {
		Ok(data) => Json(serde_json::json!({
			"data": data,
			"page": page,
			"per_page": per_page,
			"total": total,
		}))
		.into_response(),
		Err(error) => {
			tracing::error!("list approvals failed: {error}");
			StatusCode::INTERNAL_SERVER_ERROR.into_response()
		}
	}
}

pub async fn decide_approval(
	State(state): State<AppState>,
	Path(approval_id): Path<Uuid>,
	auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
	Json(body): Json<ApprovalDecisionRequest>,
) -> impl IntoResponse {
	let approval = sqlx::query_as::<_, WorkflowApproval>(
		r#"SELECT * FROM workflow_approvals WHERE id = $1"#,
	)
	.bind(approval_id)
	.fetch_optional(&state.db)
	.await;

	let Some(approval) = (match approval {
		Ok(approval) => approval,
		Err(error) => {
			tracing::error!("approval lookup failed: {error}");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	}) else {
		return StatusCode::NOT_FOUND.into_response();
	};

	if approval.status != "pending" {
		return (
			StatusCode::BAD_REQUEST,
			Json(serde_json::json!({ "error": "approval is not pending" })),
		)
			.into_response();
	}

	let updated_approval = sqlx::query_as::<_, WorkflowApproval>(
		r#"UPDATE workflow_approvals
		   SET status = CASE WHEN LOWER($2) = 'approved' THEN 'approved' ELSE 'rejected' END,
			   decision = $2,
			   payload = $3,
			   decided_at = NOW(),
			   decided_by = $4
		   WHERE id = $1
		   RETURNING *"#,
	)
	.bind(approval_id)
	.bind(&body.decision)
	.bind(&body.payload)
	.bind(claims.sub)
	.fetch_one(&state.db)
	.await;

	let Ok(updated_approval) = updated_approval else {
		tracing::error!("approval update failed");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	};

	let workflow = match sqlx::query_as::<_, WorkflowDefinition>(
		r#"SELECT * FROM workflows WHERE id = $1"#,
	)
	.bind(updated_approval.workflow_id)
	.fetch_one(&state.db)
	.await
	{
		Ok(workflow) => workflow,
		Err(error) => {
			tracing::error!("workflow lookup for approval failed: {error}");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let run = match sqlx::query_as::<_, WorkflowRun>(
		r#"SELECT * FROM workflow_runs WHERE id = $1"#,
	)
	.bind(updated_approval.workflow_run_id)
	.fetch_one(&state.db)
	.await
	{
		Ok(run) => run,
		Err(error) => {
			tracing::error!("run lookup for approval failed: {error}");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let steps = match workflow.parsed_steps() {
		Ok(steps) => steps,
		Err(error) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(serde_json::json!({ "error": error })),
			)
				.into_response();
		}
	};
	let Some(step): Option<&WorkflowStep> = steps.iter().find(|step| step.id == updated_approval.step_id) else {
		return (
			StatusCode::BAD_REQUEST,
			Json(serde_json::json!({ "error": "approval step not found in workflow" })),
		)
			.into_response();
	};

	let mut context = run.context.clone();
	executor::insert_approval_decision(
		&mut context,
		&updated_approval.step_id,
		&body.decision,
		claims.sub,
		&body.payload,
		body.comment.as_deref(),
	);

	let run = match sqlx::query_as::<_, WorkflowRun>(
		r#"UPDATE workflow_runs SET context = $2 WHERE id = $1 RETURNING *"#,
	)
	.bind(run.id)
	.bind(&context)
	.fetch_one(&state.db)
	.await
	{
		Ok(run) => run,
		Err(error) => {
			tracing::error!("run context update failed: {error}");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	match executor::continue_after_approval(&state, &workflow, run, &body.decision, step).await {
		Ok(updated_run) => Json(serde_json::json!({
			"approval": updated_approval,
			"run": updated_run,
		}))
		.into_response(),
		Err(error) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(serde_json::json!({ "error": error })),
		)
			.into_response(),
	}
}
