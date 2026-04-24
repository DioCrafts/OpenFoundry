use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::ci,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_ci_runs, load_commits,
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
    load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let commits = load_commits(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
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
    load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let commits = load_commits(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let parent_sha = commits
        .iter()
        .find(|commit| commit.branch_name == request.branch_name)
        .map(|commit| commit.sha.clone());
    let sequence = commits.len() + 1;
    let sha = format!("{:07x}", (id.as_u128() as usize + sequence) % 0x0fff_fff);
    let now = Utc::now();
    let files_changed = crate::domain::git::commit_files_changed(&request.title) as i32;

    sqlx::query(
		"INSERT INTO code_repository_commits (id, repository_id, branch_name, sha, parent_sha, title, description, author_name, author_email, files_changed, additions, deletions, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
	)
	.bind(uuid::Uuid::now_v7())
	.bind(id)
	.bind(&request.branch_name)
	.bind(&sha)
	.bind(parent_sha)
	.bind(&request.title)
	.bind(&request.description)
	.bind(&request.author_name)
	.bind(crate::domain::git::synthetic_signature(&request.author_name))
	.bind(files_changed)
	.bind(request.additions)
	.bind(request.deletions)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    sqlx::query(
		"UPDATE code_repository_branches SET head_sha = $2, ahead_by = ahead_by + 1, updated_at = $3 WHERE repository_id = $1 AND name = $4",
	)
	.bind(id)
	.bind(&sha)
	.bind(now)
	.bind(&request.branch_name)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let commits = load_commits(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let commit = commits
        .into_iter()
        .find(|entry| entry.sha == sha)
        .ok_or_else(|| internal_error("created commit could not be reloaded"))?;
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
    load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let commits = load_commits(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let run = ci::simulate_ci_run(
        id,
        &request.branch_name,
        commits
            .iter()
            .find(|commit| commit.branch_name == request.branch_name),
    );
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
