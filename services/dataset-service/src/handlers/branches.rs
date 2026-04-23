use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    models::{
        branch::{CreateDatasetBranchRequest, DatasetBranch},
        dataset::Dataset,
    },
    AppState,
};

pub async fn list_branches(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("branch dataset lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_default_branch(&state, &dataset).await {
        tracing::error!("ensure default branch failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    match sqlx::query_as::<_, DatasetBranch>(
        r#"SELECT * FROM dataset_branches
           WHERE dataset_id = $1
           ORDER BY is_default DESC, name ASC"#,
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(branches) => Json(branches).into_response(),
        Err(error) => {
            tracing::error!("list branches failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_branch(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(body): Json<CreateDatasetBranchRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "branch name is required" }))).into_response();
    }

    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("create branch dataset lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_default_branch(&state, &dataset).await {
        tracing::error!("ensure default branch failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let source_version = body.source_version.unwrap_or(dataset.current_version);
    let version_exists = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS(
               SELECT 1 FROM dataset_versions WHERE dataset_id = $1 AND version = $2
           )"#,
    )
    .bind(dataset_id)
    .bind(source_version)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if source_version != dataset.current_version && !version_exists {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "source version does not exist" })),
        )
            .into_response();
    }

    let result = sqlx::query_as::<_, DatasetBranch>(
        r#"INSERT INTO dataset_branches (
               id, dataset_id, name, version, description, is_default
           )
           VALUES ($1, $2, $3, $4, $5, FALSE)
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(dataset_id)
    .bind(body.name.trim())
    .bind(source_version)
    .bind(body.description.unwrap_or_default())
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(branch) => (StatusCode::CREATED, Json(branch)).into_response(),
        Err(error) => {
            tracing::error!("create branch failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": error.to_string() }))).into_response()
        }
    }
}

pub async fn checkout_branch(
    State(state): State<AppState>,
    Path((dataset_id, branch_name)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("checkout branch dataset lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_default_branch(&state, &dataset).await {
        tracing::error!("ensure default branch failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let branch = match sqlx::query_as::<_, DatasetBranch>(
        r#"SELECT * FROM dataset_branches
           WHERE dataset_id = $1 AND name = $2"#,
    )
    .bind(dataset_id)
    .bind(&branch_name)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(branch)) => branch,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("checkout branch query failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match sqlx::query_as::<_, Dataset>(
        r#"UPDATE datasets
           SET active_branch = $2,
               current_version = $3,
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(dataset_id)
    .bind(&branch.name)
    .bind(branch.version)
    .fetch_one(&state.db)
    .await
    {
        Ok(dataset) => Json(dataset).into_response(),
        Err(error) => {
            tracing::error!("checkout branch update failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn ensure_default_branch(state: &AppState, dataset: &Dataset) -> Result<(), sqlx::Error> {
    let has_branches = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS(SELECT 1 FROM dataset_branches WHERE dataset_id = $1)"#,
    )
    .bind(dataset.id)
    .fetch_one(&state.db)
    .await?;

    if !has_branches {
        sqlx::query(
            r#"INSERT INTO dataset_branches (
                   id, dataset_id, name, version, description, is_default
               )
               VALUES ($1, $2, 'main', $3, 'Default branch', TRUE)"#,
        )
        .bind(Uuid::now_v7())
        .bind(dataset.id)
        .bind(dataset.current_version)
        .execute(&state.db)
        .await?;
    }

    Ok(())
}

async fn load_dataset(
    state: &AppState,
    dataset_id: Uuid,
) -> Result<Option<Dataset>, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
}