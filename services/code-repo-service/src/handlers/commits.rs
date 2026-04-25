use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_ci_runs,
        load_repository_row, not_found,
    },
    models::{
        ListResponse,
        commit::{CiRun, CommitDefinition, CreateCommitRequest, TriggerCiRunRequest},
    },
};

pub async fn list_commits(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<CommitDefinition>> {
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let commits = crate::domain::git::list_commits(&state.repo_storage_root, &repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(ListResponse { items: commits }))
}

pub async fn create_commit(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateCommitRequest>,
) -> ServiceResult<CommitDefinition> {
    if request.title.trim().is_empty() {
        return Err(bad_request("commit title is required"));
    }
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let commit = crate::domain::git::apply_commit(&state.repo_storage_root, &repository, &request)
        .map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(commit))
}

pub async fn list_ci_runs(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<CiRun>> {
    load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let runs = load_ci_runs(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: runs }))
}

pub async fn trigger_ci_run(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<TriggerCiRunRequest>,
) -> ServiceResult<CiRun> {
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let run =
        crate::domain::git::run_ci_for_repository(&state.repo_storage_root, &repository, &request.branch_name)
            .map_err(|cause| internal_error(cause.to_string()))?;
    let checks =
        serde_json::to_value(&run.checks).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO code_ci_runs (id, repository_id, branch_name, commit_sha, pipeline_name, status, trigger, started_at, completed_at, checks)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)",
	)
	.bind(run.id)
	.bind(run.repository_id)
	.bind(&run.branch_name)
	.bind(&run.commit_sha)
	.bind(&run.pipeline_name)
	.bind(&run.status)
	.bind(&run.trigger)
	.bind(run.started_at)
	.bind(run.completed_at)
	.bind(checks)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    Ok(Json(run))
}
