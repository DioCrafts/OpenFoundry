use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    AppState,
    domain::review,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_comments,
        load_merge_request_row, load_merge_requests, load_repository_row, not_found,
    },
    models::{
        ListResponse,
        comment::{CreateCommentRequest, ReviewComment},
        merge_request::{
            CreateMergeRequestRequest, MergeRequestDefinition, MergeRequestDetail,
            MergeRequestStatus, UpdateMergeRequestRequest,
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
    load_repository_row(&state.db, request.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let labels =
        serde_json::to_value(&request.labels).map_err(|cause| internal_error(cause.to_string()))?;
    let reviewers = serde_json::to_value(&request.reviewers)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO code_merge_requests (id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at)
		 VALUES ($1, $2, $3, $4, $5, $6, 'open', $7, $8::jsonb, $9::jsonb, $10, $11, $12, $13, NULL)",
	)
	.bind(id)
	.bind(request.repository_id)
	.bind(&request.title)
	.bind(&request.description)
	.bind(&request.source_branch)
	.bind(&request.target_branch)
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

    if let Some(title) = request.title {
        merge_request.title = title;
    }
    if let Some(description) = request.description {
        merge_request.description = description;
    }
    if let Some(status) = request.status {
        merge_request.status = status;
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

    let now = Utc::now();
    let labels = serde_json::to_value(&merge_request.labels)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let reviewers = serde_json::to_value(&merge_request.reviewers)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let merged_at = if merge_request.status == MergeRequestStatus::Merged {
        Some(now)
    } else {
        merge_request.merged_at
    };

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
