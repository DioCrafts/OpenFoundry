use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_branches, load_commits,
        load_repository_row, not_found,
    },
    models::{
        ListResponse,
        branch::{BranchDefinition, CreateBranchRequest},
    },
};

pub async fn list_branches(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<BranchDefinition>> {
    load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let branches = load_branches(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: branches }))
}

pub async fn create_branch(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateBranchRequest>,
) -> ServiceResult<BranchDefinition> {
    if request.name.trim().is_empty() {
        return Err(bad_request("branch name is required"));
    }
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let commits = load_commits(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let now = Utc::now();
    let head_sha = commits
        .iter()
        .find(|commit| commit.branch_name == request.base_branch)
        .map(|commit| commit.sha.clone())
        .unwrap_or_else(|| "init000".to_string());
    let (ahead_by, pending_reviews) = crate::domain::git::branch_metrics(
        &BranchDefinition {
            id: uuid::Uuid::nil(),
            repository_id: id,
            name: request.name.clone(),
            head_sha: head_sha.clone(),
            base_branch: Some(request.base_branch.clone()),
            is_default: false,
            protected: request.protected,
            ahead_by: 0,
            pending_reviews: 0,
            updated_at: now,
        },
        commits.len(),
    );

    sqlx::query(
		"INSERT INTO code_repository_branches (id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at)
		 VALUES ($1, $2, $3, $4, $5, false, $6, $7, $8, $9)",
	)
	.bind(uuid::Uuid::now_v7())
	.bind(repository.id)
	.bind(&request.name)
	.bind(&head_sha)
	.bind(&request.base_branch)
	.bind(request.protected)
	.bind(ahead_by)
	.bind(pending_reviews as i32)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let branches = load_branches(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let branch = branches
        .into_iter()
        .find(|entry| entry.name == request.name)
        .ok_or_else(|| internal_error("created branch could not be reloaded"))?;
    Ok(Json(branch))
}
