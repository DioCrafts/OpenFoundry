pub mod branches;
pub mod commits;
pub mod diff;
pub mod files;
pub mod integrations;
pub mod merge_requests;
pub mod repos;

use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::models::{
    branch::BranchRow,
    comment::CommentRow,
    commit::{CiRun, CiRunRow},
    integration::{IntegrationRow, SyncRunRow},
    merge_request::MergeRequestRow,
    repository::RepositoryRow,
};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("code-repo-service database error: {cause}");
    internal_error("database operation failed")
}

pub async fn load_repository_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<RepositoryRow>, sqlx::Error> {
    sqlx::query_as::<_, RepositoryRow>(
		"SELECT id, name, slug, description, owner, default_branch, visibility, object_store_backend, package_kind, tags, settings, created_at, updated_at
		 FROM code_repositories
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_all_repositories(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::repository::RepositoryDefinition>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RepositoryRow>(
		"SELECT id, name, slug, description, owner, default_branch, visibility, object_store_backend, package_kind, tags, settings, created_at, updated_at
		 FROM code_repositories
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::repository::RepositoryDefinition::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_merge_request_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<MergeRequestRow>, sqlx::Error> {
    sqlx::query_as::<_, MergeRequestRow>(
		"SELECT id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at
		 FROM code_merge_requests
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_merge_requests(
    db: &sqlx::PgPool,
    repository_id: Option<uuid::Uuid>,
) -> Result<Vec<crate::models::merge_request::MergeRequestDefinition>, sqlx::Error> {
    let rows = if let Some(repository_id) = repository_id {
        sqlx::query_as::<_, MergeRequestRow>(
			"SELECT id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at
			 FROM code_merge_requests
			 WHERE repository_id = $1
			 ORDER BY updated_at DESC",
		)
		.bind(repository_id)
		.fetch_all(db)
		.await?
    } else {
        sqlx::query_as::<_, MergeRequestRow>(
			"SELECT id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at
			 FROM code_merge_requests
			 ORDER BY updated_at DESC",
		)
		.fetch_all(db)
		.await?
    };

    rows.into_iter()
        .map(crate::models::merge_request::MergeRequestDefinition::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_comments(
    db: &sqlx::PgPool,
    merge_request_id: uuid::Uuid,
) -> Result<Vec<crate::models::comment::ReviewComment>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CommentRow>(
        "SELECT id, merge_request_id, author, body, file_path, line_number, resolved, created_at
		 FROM code_review_comments
		 WHERE merge_request_id = $1
		 ORDER BY created_at ASC",
    )
    .bind(merge_request_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(crate::models::comment::ReviewComment::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_ci_runs(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
) -> Result<Vec<crate::models::commit::CiRun>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CiRunRow>(
		"SELECT id, repository_id, branch_name, commit_sha, pipeline_name, status, trigger, started_at, completed_at, checks
		 FROM code_ci_runs
		 WHERE repository_id = $1
		 ORDER BY started_at DESC",
	)
	.bind(repository_id)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::commit::CiRun::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_branch_row(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
    branch_name: &str,
) -> Result<Option<BranchRow>, sqlx::Error> {
    sqlx::query_as::<_, BranchRow>(
        "SELECT id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at
         FROM code_repository_branches
         WHERE repository_id = $1 AND name = $2",
    )
    .bind(repository_id)
    .bind(branch_name)
    .fetch_optional(db)
    .await
}

pub async fn persist_ci_run(db: &sqlx::PgPool, run: &CiRun) -> Result<(), sqlx::Error> {
    let checks = serde_json::to_value(&run.checks).map_err(|cause| {
        sqlx::Error::Decode(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            cause,
        )))
    })?;

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
    .execute(db)
    .await?;

    Ok(())
}

pub async fn sync_branch_head(
    db: &sqlx::PgPool,
    repository_id: uuid::Uuid,
    branch_name: &str,
    head_sha: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE code_repository_branches
         SET head_sha = $3,
             updated_at = NOW()
         WHERE repository_id = $1
           AND name = $2",
    )
    .bind(repository_id)
    .bind(branch_name)
    .bind(head_sha)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn load_integrations(
    db: &sqlx::PgPool,
    repository_id: Option<uuid::Uuid>,
) -> Result<Vec<crate::models::integration::RepositoryIntegration>, sqlx::Error> {
    let rows = if let Some(repository_id) = repository_id {
        sqlx::query_as::<_, IntegrationRow>(
			"SELECT id, repository_id, provider, external_namespace, external_project, external_url, sync_mode, ci_trigger_strategy, status, default_branch, branch_mapping, webhook_url, last_synced_at, created_at, updated_at
			 FROM code_repository_integrations
			 WHERE repository_id = $1
			 ORDER BY updated_at DESC",
		)
		.bind(repository_id)
		.fetch_all(db)
		.await?
    } else {
        sqlx::query_as::<_, IntegrationRow>(
			"SELECT id, repository_id, provider, external_namespace, external_project, external_url, sync_mode, ci_trigger_strategy, status, default_branch, branch_mapping, webhook_url, last_synced_at, created_at, updated_at
			 FROM code_repository_integrations
			 ORDER BY updated_at DESC",
		)
		.fetch_all(db)
		.await?
    };

    rows.into_iter()
        .map(crate::models::integration::RepositoryIntegration::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_integration_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<IntegrationRow>, sqlx::Error> {
    sqlx::query_as::<_, IntegrationRow>(
		"SELECT id, repository_id, provider, external_namespace, external_project, external_url, sync_mode, ci_trigger_strategy, status, default_branch, branch_mapping, webhook_url, last_synced_at, created_at, updated_at
		 FROM code_repository_integrations
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_sync_runs(
    db: &sqlx::PgPool,
    integration_id: uuid::Uuid,
) -> Result<Vec<crate::models::integration::ExternalSyncRun>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SyncRunRow>(
		"SELECT id, integration_id, repository_id, trigger, status, commit_sha, branch_name, summary, checks, started_at, completed_at
		 FROM code_repository_sync_runs
		 WHERE integration_id = $1
		 ORDER BY started_at DESC",
	)
	.bind(integration_id)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::integration::ExternalSyncRun::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}
