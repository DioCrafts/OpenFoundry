use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_branch_row, load_ci_runs,
        load_repository_row, not_found, persist_ci_run, sync_branch_head,
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
    let protected_branch = load_branch_row(&state.db, id, &request.branch_name)
        .await
        .map_err(|cause| db_error(&cause))?
        .map(|branch| branch.protected)
        .unwrap_or(request.branch_name == repository.default_branch);
    if protected_branch && !allow_direct_commits_on_protected(&repository) {
        return Err(bad_request(format!(
            "branch '{}' is protected; create a merge request from a writable branch instead",
            request.branch_name
        )));
    }
    let commit = crate::domain::git::apply_commit(&state.repo_storage_root, &repository, &request)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let head_sha = crate::domain::git::branch_head_sha(
        &state.repo_storage_root,
        repository.id,
        &request.branch_name,
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
    sync_branch_head(&state.db, repository.id, &request.branch_name, &head_sha)
        .await
        .map_err(|cause| db_error(&cause))?;

    if repository.ci_required() {
        let run = crate::domain::git::run_ci_for_repository_with_trigger(
            &state.repo_storage_root,
            &repository,
            &request.branch_name,
            "push",
        )
        .map_err(|cause| internal_error(cause.to_string()))?;
        persist_ci_run(&state.db, &run)
            .await
            .map_err(|cause| db_error(&cause))?;
    }
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
    let run = crate::domain::git::run_ci_for_repository(
        &state.repo_storage_root,
        &repository,
        &request.branch_name,
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
    persist_ci_run(&state.db, &run)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(run))
}

fn allow_direct_commits_on_protected(
    repository: &crate::models::repository::RepositoryDefinition,
) -> bool {
    repository.allow_direct_commits_on_protected()
}
