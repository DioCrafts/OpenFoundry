use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_integration_row,
        load_integrations, load_repository_row, load_sync_runs, not_found,
    },
    models::{
        ListResponse,
        integration::{
            CreateIntegrationRequest, ExternalSyncRun, IntegrationDetail, RepositoryIntegration,
            TriggerSyncRequest, UpdateIntegrationRequest,
        },
    },
};

#[derive(Debug, Deserialize)]
pub struct IntegrationQuery {
    pub repository_id: Option<uuid::Uuid>,
}

pub async fn list_integrations(
    Query(query): Query<IntegrationQuery>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<RepositoryIntegration>> {
    let items = load_integrations(&state.db, query.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items }))
}

pub async fn get_integration(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<IntegrationDetail> {
    let row = load_integration_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("integration not found"))?;
    let integration =
        RepositoryIntegration::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    let sync_runs = load_sync_runs(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(IntegrationDetail {
        integration,
        sync_runs,
    }))
}

pub async fn create_integration(
    State(state): State<AppState>,
    Json(request): Json<CreateIntegrationRequest>,
) -> ServiceResult<RepositoryIntegration> {
    if request.external_project.trim().is_empty() {
        return Err(bad_request("external project is required"));
    }

    if load_repository_row(&state.db, request.repository_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .is_none()
    {
        return Err(bad_request("repository not found"));
    }

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let branch_mapping = serde_json::to_value(&request.branch_mapping)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO code_repository_integrations (id, repository_id, provider, external_namespace, external_project, external_url, sync_mode, ci_trigger_strategy, status, default_branch, branch_mapping, webhook_url, last_synced_at, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13, $14, $15)",
	)
	.bind(id)
	.bind(request.repository_id)
	.bind(request.provider.as_str())
	.bind(&request.external_namespace)
	.bind(&request.external_project)
	.bind(&request.external_url)
	.bind(&request.sync_mode)
	.bind(&request.ci_trigger_strategy)
	.bind("connected")
	.bind(&request.default_branch)
	.bind(branch_mapping)
	.bind(&request.webhook_url)
	.bind(Option::<chrono::DateTime<chrono::Utc>>::None)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_integration_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created integration could not be reloaded"))?;
    let integration =
        RepositoryIntegration::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(integration))
}

pub async fn update_integration(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateIntegrationRequest>,
) -> ServiceResult<RepositoryIntegration> {
    let current = load_integration_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("integration not found"))?;
    let current = RepositoryIntegration::try_from(current)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let branch_mapping = serde_json::to_value(
        request
            .branch_mapping
            .clone()
            .unwrap_or(current.branch_mapping.clone()),
    )
    .map_err(|cause| internal_error(cause.to_string()))?;
    let now = Utc::now();

    sqlx::query(
        "UPDATE code_repository_integrations
		 SET external_namespace = $2,
			 external_project = $3,
			 external_url = $4,
			 sync_mode = $5,
			 ci_trigger_strategy = $6,
			 status = $7,
			 default_branch = $8,
			 branch_mapping = $9::jsonb,
			 webhook_url = $10,
			 updated_at = $11
		 WHERE id = $1",
    )
    .bind(id)
    .bind(
        request
            .external_namespace
            .unwrap_or(current.external_namespace),
    )
    .bind(request.external_project.unwrap_or(current.external_project))
    .bind(request.external_url.unwrap_or(current.external_url))
    .bind(request.sync_mode.unwrap_or(current.sync_mode))
    .bind(
        request
            .ci_trigger_strategy
            .unwrap_or(current.ci_trigger_strategy),
    )
    .bind(request.status.unwrap_or(current.status))
    .bind(request.default_branch.unwrap_or(current.default_branch))
    .bind(branch_mapping)
    .bind(request.webhook_url.unwrap_or(current.webhook_url))
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_integration_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated integration could not be reloaded"))?;
    let integration =
        RepositoryIntegration::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(integration))
}

pub async fn list_sync_runs(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ExternalSyncRun>> {
    let items = load_sync_runs(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items }))
}

pub async fn trigger_sync(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<TriggerSyncRequest>,
) -> ServiceResult<ExternalSyncRun> {
    let integration_row = load_integration_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("integration not found"))?;
    let integration = RepositoryIntegration::try_from(integration_row)
        .map_err(|cause| internal_error(cause.to_string()))?;

    if request.commit_sha.trim().is_empty() {
        return Err(bad_request("commit sha is required"));
    }

    let sync_id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let summary = format!(
        "{} sync queued for {} / {} on branch {}",
        integration.provider.label(),
        integration.external_namespace,
        integration.external_project,
        request.branch_name,
    );
    let checks = serde_json::json!([
        format!("mirror:{}", integration.sync_mode),
        format!("ci:{}", integration.ci_trigger_strategy),
        format!("provider:{}", integration.provider.as_str()),
    ]);

    sqlx::query(
		"INSERT INTO code_repository_sync_runs (id, integration_id, repository_id, trigger, status, commit_sha, branch_name, summary, checks, started_at, completed_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10, $11)",
	)
	.bind(sync_id)
	.bind(id)
	.bind(integration.repository_id)
	.bind(request.trigger)
	.bind("completed")
	.bind(&request.commit_sha)
	.bind(&request.branch_name)
	.bind(summary)
	.bind(checks)
	.bind(now)
	.bind(Some(now + Duration::minutes(4)))
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    sqlx::query(
        "UPDATE code_repository_integrations
		 SET status = $2,
			 last_synced_at = $3,
			 updated_at = $4
		 WHERE id = $1",
    )
    .bind(id)
    .bind("connected")
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let sync_run = load_sync_runs(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .find(|run| run.id == sync_id)
        .ok_or_else(|| internal_error("created sync run could not be reloaded"))?;
    Ok(Json(sync_run))
}
