use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;

use crate::{
	domain::search,
	handlers::{db_error, load_files, load_repository_row, not_found, ServiceResult},
	models::{file::{RepositoryFile, SearchResponse}, ListResponse},
	AppState,
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
	pub q: Option<String>,
}

pub async fn list_files(
	Path(id): Path<uuid::Uuid>,
	State(state): State<AppState>,
) -> ServiceResult<ListResponse<RepositoryFile>> {
	load_repository_row(&state.db, id).await.map_err(|cause| db_error(&cause))?.ok_or_else(|| not_found("repository not found"))?;
	let files = load_files(&state.db, id).await.map_err(|cause| db_error(&cause))?;
	Ok(Json(ListResponse { items: files }))
}

pub async fn search_files(
	Path(id): Path<uuid::Uuid>,
	Query(query): Query<SearchQuery>,
	State(state): State<AppState>,
) -> ServiceResult<SearchResponse> {
	load_repository_row(&state.db, id).await.map_err(|cause| db_error(&cause))?.ok_or_else(|| not_found("repository not found"))?;
	let files = load_files(&state.db, id).await.map_err(|cause| db_error(&cause))?;
	let query_text = query.q.unwrap_or_else(|| "package".to_string());
	let results = search::search(&files, &query_text);
	Ok(Json(SearchResponse { query: query_text, results }))
}