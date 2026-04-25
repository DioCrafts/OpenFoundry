use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_merge_requests,
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
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let metadata = branch_metadata(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let pending_reviews = pending_review_counts(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let branches = crate::domain::git::list_branches(
        &state.repo_storage_root,
        &repository,
        &metadata,
        &pending_reviews,
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
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
    crate::domain::git::create_branch(
        &state.repo_storage_root,
        repository.id,
        &request.name,
        &request.base_branch,
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
    let now = Utc::now();
    let metadata_id = uuid::Uuid::now_v7();

    sqlx::query(
		"INSERT INTO code_repository_branches (id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at)
		 VALUES ($1, $2, $3, $4, $5, false, $6, $7, $8, $9)",
	)
	.bind(metadata_id)
	.bind(repository.id)
	.bind(&request.name)
	.bind("")
	.bind(&request.base_branch)
	.bind(request.protected)
	.bind(0)
	.bind(0)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let metadata = branch_metadata(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let pending_reviews = pending_review_counts(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let branches = crate::domain::git::list_branches(
        &state.repo_storage_root,
        &repository,
        &metadata,
        &pending_reviews,
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
    let branch = branches
        .into_iter()
        .find(|entry| entry.name == request.name)
        .ok_or_else(|| internal_error("created branch could not be reloaded"))?;
    Ok(Json(branch))
}

async fn branch_metadata(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
) -> Result<std::collections::BTreeMap<String, crate::domain::git::GitBranchMetadata>, sqlx::Error>
{
    let rows = sqlx::query_as::<_, crate::models::branch::BranchRow>(
        "SELECT id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at
         FROM code_repository_branches
         WHERE repository_id = $1",
    )
    .bind(repository_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.name,
                crate::domain::git::GitBranchMetadata {
                    id: row.id,
                    base_branch: row.base_branch,
                    protected: row.protected,
                },
            )
        })
        .collect())
}

async fn pending_review_counts(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
) -> Result<std::collections::BTreeMap<String, usize>, sqlx::Error> {
    let merge_requests = load_merge_requests(db, Some(repository_id)).await?;
    let mut counts = std::collections::BTreeMap::new();
    for merge_request in merge_requests.into_iter().filter(|merge_request| {
        merge_request.status == crate::models::merge_request::MergeRequestStatus::Open
    }) {
        *counts.entry(merge_request.source_branch).or_insert(0) += 1;
    }
    Ok(counts)
}
