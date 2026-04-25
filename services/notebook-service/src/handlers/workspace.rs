use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::environment,
    models::workspace::{
        DeleteNotebookWorkspaceFileQuery, ListNotebookWorkspaceFilesResponse,
        UpsertNotebookWorkspaceFileRequest,
    },
};

pub async fn list_workspace_files(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
) -> impl IntoResponse {
    match environment::list_workspace_files(&state.data_dir, notebook_id).await {
        Ok(files) => Json(ListNotebookWorkspaceFilesResponse { data: files }).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    }
}

pub async fn upsert_workspace_file(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
    Json(body): Json<UpsertNotebookWorkspaceFileRequest>,
) -> impl IntoResponse {
    match environment::upsert_workspace_file(
        &state.data_dir,
        notebook_id,
        &body.path,
        &body.content,
    )
    .await
    {
        Ok(file) => Json(serde_json::json!(file)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn delete_workspace_file(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
    Query(query): Query<DeleteNotebookWorkspaceFileQuery>,
) -> impl IntoResponse {
    match environment::delete_workspace_file(&state.data_dir, notebook_id, &query.path).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}
