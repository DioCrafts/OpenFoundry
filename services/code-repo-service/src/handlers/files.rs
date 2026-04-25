use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{
    AppState,
    domain::search,
    handlers::{ServiceResult, db_error, load_repository_row, not_found},
    models::{
        ListResponse,
        file::{RepositoryFile, SearchResponse},
    },
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn list_files(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<RepositoryFile>> {
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    let files = crate::domain::git::list_files(
        &state.repo_storage_root,
        repository.id,
        &repository.default_branch,
    )
    .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    Ok(Json(ListResponse { items: files }))
}

pub async fn search_files(
    Path(id): Path<uuid::Uuid>,
    Query(query): Query<SearchQuery>,
    State(state): State<AppState>,
) -> ServiceResult<SearchResponse> {
    let repository = load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("repository not found"))?;
    let repository = crate::models::repository::RepositoryDefinition::try_from(repository)
        .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    let files = crate::domain::git::list_files(
        &state.repo_storage_root,
        repository.id,
        &repository.default_branch,
    )
    .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    let query_text = query.q.unwrap_or_else(|| "package".to_string());
    let results = search::search(&files, &query_text);
    Ok(Json(SearchResponse {
        query: query_text,
        results,
    }))
}
