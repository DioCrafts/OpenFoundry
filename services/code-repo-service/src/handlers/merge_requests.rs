use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashSet;

use crate::{
    AppState,
    domain::review,
    handlers::{
        ErrorResponse, ServiceResult, bad_request, db_error, internal_error, load_branch_row,
        load_ci_runs, load_comments, load_merge_request_row, load_merge_requests,
        load_repository_row, not_found, persist_ci_run, sync_branch_head,
    },
    models::{
        ListResponse,
        comment::{CreateCommentRequest, ReviewComment},
        merge_request::{
            CreateMergeRequestRequest, MergeMergeRequestRequest, MergeRequestDefinition,
            MergeRequestDetail, MergeRequestMergeResult, MergeRequestStatus,
            UpdateMergeRequestRequest,
        },
    },
};

#[derive(Debug, Deserialize)]
pub struct MergeRequestQuery {
    pub repository_id: Option<uuid::Uuid>,
}

pub async fn list_merge_requests(
    Query(query): Query<MergeRequestQuery>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<MergeRequestDefinition>> {
    let merge_requests = load_merge_requests(&state.db, query.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse {
        items: merge_requests,
    }))
}

pub async fn create_merge_request(
    State(state): State<AppState>,
    Json(request): Json<CreateMergeRequestRequest>,
) -> ServiceResult<MergeRequestDefinition> {
    if request.title.trim().is_empty() {
        return Err(bad_request("merge request title is required"));
    }
    let repository = load_repository_row(&state.db, request.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    if request.source_branch == request.target_branch {
        return Err(bad_request("source and target branches must be different"));
    }
    crate::domain::git::branch_head_sha(
        &state.repo_storage_root,
        repository.id,
        &request.source_branch,
    )
    .map_err(|cause| bad_request(cause.to_string()))?;
    crate::domain::git::branch_head_sha(
        &state.repo_storage_root,
        repository.id,
        &request.target_branch,
    )
    .map_err(|cause| bad_request(cause.to_string()))?;
    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let labels =
        serde_json::to_value(&request.labels).map_err(|cause| internal_error(cause.to_string()))?;
    let reviewers = serde_json::to_value(&request.reviewers)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let status =
        effective_merge_request_status(None, &request.reviewers, request.approvals_required)
            .map_err(bad_request)?;

    sqlx::query(
		"INSERT INTO code_merge_requests (id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, $11, $12, $13, $14, NULL)",
	)
	.bind(id)
	.bind(request.repository_id)
	.bind(&request.title)
	.bind(&request.description)
	.bind(&request.source_branch)
	.bind(&request.target_branch)
	.bind(status.as_str())
	.bind(&request.author)
	.bind(labels)
	.bind(reviewers)
	.bind(request.approvals_required)
	.bind(request.changed_files)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created merge request could not be reloaded"))?;
    let merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(merge_request))
}

pub async fn get_merge_request(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<MergeRequestDetail> {
    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("merge request not found"))?;
    let merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    let comments = load_comments(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let (approvals, threads) = review::approval_summary(&merge_request, &comments);
    Ok(Json(MergeRequestDetail {
        merge_request,
        comments,
        approval_count: approvals,
        thread_count: threads,
    }))
}

pub async fn update_merge_request(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateMergeRequestRequest>,
) -> ServiceResult<MergeRequestDefinition> {
    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("merge request not found"))?;
    let mut merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    if merge_request.status == MergeRequestStatus::Merged {
        return Err(bad_request("merged merge requests are immutable"));
    }

    if let Some(title) = request.title {
        merge_request.title = title;
    }
    if let Some(description) = request.description {
        merge_request.description = description;
    }
    if let Some(labels) = request.labels {
        merge_request.labels = labels;
    }
    if let Some(reviewers) = request.reviewers {
        merge_request.reviewers = reviewers;
    }
    if let Some(approvals_required) = request.approvals_required {
        merge_request.approvals_required = approvals_required;
    }
    if let Some(changed_files) = request.changed_files {
        merge_request.changed_files = changed_files;
    }
    merge_request.status =
        if request.status.is_none() && merge_request.status == MergeRequestStatus::Closed {
            MergeRequestStatus::Closed
        } else {
            effective_merge_request_status(
                request.status,
                &merge_request.reviewers,
                merge_request.approvals_required,
            )
            .map_err(bad_request)?
        };

    let now = Utc::now();
    let labels = serde_json::to_value(&merge_request.labels)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let reviewers = serde_json::to_value(&merge_request.reviewers)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let merged_at = merge_request.merged_at;

    sqlx::query(
		"UPDATE code_merge_requests
		 SET title = $2, description = $3, status = $4, labels = $5::jsonb, reviewers = $6::jsonb, approvals_required = $7, changed_files = $8, updated_at = $9, merged_at = $10
		 WHERE id = $1",
	)
	.bind(id)
	.bind(&merge_request.title)
	.bind(&merge_request.description)
	.bind(merge_request.status.as_str())
	.bind(labels)
	.bind(reviewers)
	.bind(merge_request.approvals_required)
	.bind(merge_request.changed_files)
	.bind(now)
	.bind(merged_at)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated merge request could not be reloaded"))?;
    let merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(merge_request))
}

pub async fn merge_merge_request(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<MergeMergeRequestRequest>,
) -> Result<Json<MergeRequestMergeResult>, (StatusCode, Json<ErrorResponse>)> {
    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("merge request not found"))?;
    let merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    if merge_request.status == MergeRequestStatus::Merged {
        return Err(bad_request("merge request is already merged"));
    }
    if merge_request.status == MergeRequestStatus::Closed {
        return Err(bad_request("closed merge requests cannot be merged"));
    }

    let repository = load_repository_row(&state.db, merge_request.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let comments = load_comments(&state.db, merge_request.id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let (approval_count, _) = review::approval_summary(&merge_request, &comments);
    let target_branch = load_branch_row(
        &state.db,
        merge_request.repository_id,
        &merge_request.target_branch,
    )
    .await
    .map_err(|cause| db_error(&cause))?;
    let target_protected = target_branch
        .as_ref()
        .map(|branch| branch.protected)
        .unwrap_or(merge_request.target_branch == repository.default_branch);

    if target_protected && approval_count < merge_request.approvals_required as usize {
        return Err(bad_request(format!(
            "protected branch '{}' requires {} approval(s); only {} recorded",
            merge_request.target_branch, merge_request.approvals_required, approval_count
        )));
    }

    let source_head_sha = crate::domain::git::branch_head_sha(
        &state.repo_storage_root,
        repository.id,
        &merge_request.source_branch,
    )
    .map_err(|cause| bad_request(cause.to_string()))?;
    let latest_ci = latest_branch_ci(&state.db, repository.id, &merge_request.source_branch)
        .await
        .map_err(|cause| db_error(&cause))?;
    if repository_ci_required(&repository) {
        let Some(run) = latest_ci.as_ref() else {
            return Err(bad_request(format!(
                "branch '{}' must pass CI before merge",
                merge_request.source_branch
            )));
        };
        if run.commit_sha != source_head_sha {
            return Err(bad_request(format!(
                "branch '{}' has new commits without CI on head {}",
                merge_request.source_branch, source_head_sha
            )));
        }
        if run.status != "passed" {
            return Err(bad_request(format!(
                "latest CI on '{}' is '{}' and blocks merge",
                merge_request.source_branch, run.status
            )));
        }

        let required_checks = repository.required_checks_for_branch(&merge_request.target_branch);
        if !required_checks.is_empty() {
            let available_checks = available_ci_checks(run);
            let missing_checks = required_checks
                .into_iter()
                .filter(|check| !available_checks.contains(check))
                .collect::<Vec<_>>();
            if !missing_checks.is_empty() {
                return Err(bad_request(format!(
                    "branch '{}' requires CI checks [{}] before merge; latest run is missing [{}]",
                    merge_request.target_branch,
                    available_checks
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    missing_checks.join(", ")
                )));
            }
        }
    }

    let merged_by = request
        .merged_by
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(merge_request.author.as_str());
    let merge_commit_sha = crate::domain::git::merge_branches(
        &state.repo_storage_root,
        &repository,
        &merge_request.source_branch,
        &merge_request.target_branch,
        merged_by,
    )
    .map_err(|cause| {
        let message = cause.to_string();
        if message.to_ascii_lowercase().contains("conflict") {
            (StatusCode::CONFLICT, Json(ErrorResponse { error: message }))
        } else {
            internal_error(message)
        }
    })?;
    sync_branch_head(
        &state.db,
        repository.id,
        &merge_request.target_branch,
        &merge_commit_sha,
    )
    .await
    .map_err(|cause| db_error(&cause))?;

    let ci_run = if repository_ci_required(&repository) {
        let run = crate::domain::git::run_ci_for_repository_with_trigger(
            &state.repo_storage_root,
            &repository,
            &merge_request.target_branch,
            "merge",
        )
        .map_err(|cause| internal_error(cause.to_string()))?;
        persist_ci_run(&state.db, &run)
            .await
            .map_err(|cause| db_error(&cause))?;
        Some(run)
    } else {
        None
    };

    let now = Utc::now();
    sqlx::query(
        "UPDATE code_merge_requests
         SET status = 'merged',
             updated_at = $2,
             merged_at = $2
         WHERE id = $1",
    )
    .bind(merge_request.id)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("merged merge request could not be reloaded"))?;
    let merge_request =
        MergeRequestDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    let target_branch_name = merge_request.target_branch.clone();

    Ok(Json(MergeRequestMergeResult {
        merge_request,
        merge_commit_sha,
        target_branch: target_branch_name,
        ci_run,
    }))
}

pub async fn list_comments(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ReviewComment>> {
    load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("merge request not found"))?;
    let comments = load_comments(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: comments }))
}

pub async fn create_comment(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateCommentRequest>,
) -> ServiceResult<ReviewComment> {
    if request.body.trim().is_empty() {
        return Err(bad_request("comment body is required"));
    }
    load_merge_request_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("merge request not found"))?;
    let comment_id = uuid::Uuid::now_v7();
    let now = Utc::now();

    sqlx::query(
		"INSERT INTO code_review_comments (id, merge_request_id, author, body, file_path, line_number, resolved, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
	)
	.bind(comment_id)
	.bind(id)
	.bind(&request.author)
	.bind(&request.body)
	.bind(&request.file_path)
	.bind(request.line_number)
	.bind(request.resolved)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let comments = load_comments(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let comment = comments
        .into_iter()
        .find(|entry| entry.id == comment_id)
        .ok_or_else(|| internal_error("created comment could not be reloaded"))?;
    Ok(Json(comment))
}

fn effective_merge_request_status(
    requested_status: Option<MergeRequestStatus>,
    reviewers: &[crate::models::merge_request::ReviewerState],
    approvals_required: i32,
) -> Result<MergeRequestStatus, String> {
    if requested_status == Some(MergeRequestStatus::Merged) {
        return Err("merge requests must be merged through the dedicated merge action".to_string());
    }
    if requested_status == Some(MergeRequestStatus::Closed) {
        return Ok(MergeRequestStatus::Closed);
    }

    let approvals = reviewers
        .iter()
        .filter(|reviewer| reviewer.approved)
        .count() as i32;
    if requested_status == Some(MergeRequestStatus::Approved) && approvals < approvals_required {
        return Err(format!(
            "merge request needs {} approval(s) before it can be marked approved",
            approvals_required
        ));
    }

    if approvals >= approvals_required {
        Ok(MergeRequestStatus::Approved)
    } else {
        Ok(MergeRequestStatus::Open)
    }
}

fn repository_ci_required(repository: &crate::models::repository::RepositoryDefinition) -> bool {
    repository.ci_required()
}

fn available_ci_checks(run: &crate::models::commit::CiRun) -> HashSet<String> {
    let mut checks = HashSet::from([run.pipeline_name.clone()]);
    checks.extend(run.checks.iter().cloned());
    checks
}

async fn latest_branch_ci(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
    branch_name: &str,
) -> Result<Option<crate::models::commit::CiRun>, sqlx::Error> {
    let runs = load_ci_runs(db, repository_id).await?;
    Ok(runs.into_iter().find(|run| run.branch_name == branch_name))
}
