use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{
    AppState,
    handlers::{ServiceResult, db_error, load_repository_row, not_found},
    models::file::DiffResponse,
};

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub branch: Option<String>,
}

pub async fn get_repository_diff(
    Path(id): Path<uuid::Uuid>,
    Query(query): Query<DiffQuery>,
    State(state): State<AppState>,
) -> ServiceResult<DiffResponse> {
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    let branch_name = query
        .branch
        .unwrap_or_else(|| repository.default_branch.clone());
    let patch = crate::domain::git::repository_diff(
        &state.repo_storage_root,
        repository.id,
        &repository.default_branch,
        &branch_name,
    )
    .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    Ok(Json(DiffResponse { branch_name, patch }))
}
