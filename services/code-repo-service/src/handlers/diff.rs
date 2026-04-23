use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;

use crate::{
	handlers::{db_error, load_commits, load_files, load_repository_row, not_found, ServiceResult},
	models::file::DiffResponse,
	AppState,
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
	let files = load_files(&state.db, id).await.map_err(|cause| db_error(&cause))?;
	let commits = load_commits(&state.db, id).await.map_err(|cause| db_error(&cause))?;
	let branch_name = query.branch.unwrap_or(repository.default_branch);
	let patch = crate::domain::git::repository_diff(&files, &branch_name, &commits);
	Ok(Json(DiffResponse { branch_name, patch }))
}