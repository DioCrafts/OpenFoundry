use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{runtime, transactions},
    models::{
        dataset::Dataset,
        view::{CreateDatasetViewRequest, DatasetView},
    },
};

#[derive(Debug, Deserialize)]
pub struct ViewPreviewQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_views(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, DatasetView>(
        "SELECT * FROM dataset_views WHERE dataset_id = $1 ORDER BY created_at DESC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(views) => Json(views).into_response(),
        Err(error) => {
            tracing::error!("list dataset views failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_view(
    State(state): State<AppState>,
    Path((dataset_id, view_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match load_view(&state, dataset_id, view_id).await {
        Ok(Some(view)) => Json(view).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get dataset view failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_view(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(body): Json<CreateDatasetViewRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "view name is required" })),
        )
            .into_response();
    }
    if body.sql.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "view SQL is required" })),
        )
            .into_response();
    }

    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("create dataset view lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let view_id = Uuid::now_v7();
    let result = sqlx::query_as::<_, DatasetView>(
        r#"INSERT INTO dataset_views (
               id, dataset_id, name, description, sql_text, source_branch, source_version,
               materialized, refresh_on_source_update, format
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'json')
           RETURNING *"#,
    )
    .bind(view_id)
    .bind(dataset_id)
    .bind(body.name.trim())
    .bind(body.description.unwrap_or_default())
    .bind(body.sql.trim())
    .bind(
        body.source_branch
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(body.source_version)
    .bind(body.materialized.unwrap_or(true))
    .bind(body.refresh_on_source_update.unwrap_or(false))
    .fetch_one(&state.db)
    .await;

    let view = match result {
        Ok(view) => view,
        Err(error) => {
            tracing::error!("create dataset view failed: {error}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    if view.materialized {
        match refresh_view_materialization(&state, &dataset, &view).await {
            Ok(refreshed) => return (StatusCode::CREATED, Json(refreshed)).into_response(),
            Err(error) => {
                tracing::error!("initial dataset view refresh failed: {error}");
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({
                        "error": "dataset view created but initial materialization failed",
                        "details": error,
                        "view_id": view.id,
                    })),
                )
                    .into_response();
            }
        }
    }

    (StatusCode::CREATED, Json(view)).into_response()
}

pub async fn refresh_view(
    State(state): State<AppState>,
    Path((dataset_id, view_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("refresh dataset view lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let view = match load_view(&state, dataset_id, view_id).await {
        Ok(Some(view)) => view,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("refresh dataset view load failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match refresh_view_materialization(&state, &dataset, &view).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": error }))).into_response(),
    }
}

pub async fn preview_view(
    State(state): State<AppState>,
    Path((dataset_id, view_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ViewPreviewQuery>,
) -> impl IntoResponse {
    let view = match load_view(&state, dataset_id, view_id).await {
        Ok(Some(view)) => view,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("preview dataset view load failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 1_000);
    let offset = query.offset.unwrap_or(0).max(0);

    if view.materialized
        && let Some(storage_path) = &view.storage_path
    {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        match state.storage.get(storage_path).await {
            Ok(bytes) => match runtime::prepare_query_context(&view.format, &bytes).await {
                Ok(prepared) => {
                    let columns = runtime::load_schema_fields(&prepared.ctx).await;
                    let total_rows = runtime::fetch_scalar_i64(
                        &prepared.ctx,
                        "SELECT COUNT(*) AS value FROM dataset",
                    )
                    .await;
                    let rows = runtime::collect_object_rows(
                        &prepared.ctx,
                        &format!("SELECT * FROM dataset LIMIT {limit} OFFSET {offset}"),
                    )
                    .await;
                    runtime::cleanup_temp_path(prepared.path).await;

                    return Json(json!({
                        "view_id": view.id,
                        "view_name": view.name,
                        "materialized": true,
                        "dataset_id": dataset_id,
                        "source_branch": view.source_branch,
                        "source_version": view.source_version,
                        "current_version": view.current_version,
                        "row_count": total_rows.unwrap_or(view.row_count),
                        "columns": columns.unwrap_or_default(),
                        "rows": rows.unwrap_or_default(),
                        "limit": limit,
                        "offset": offset,
                        "warnings": warnings,
                        "errors": errors,
                    }))
                    .into_response();
                }
                Err(error) => errors.push(error),
            },
            Err(error) => warnings.push(format!("storage read failed: {error}")),
        }

        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": "failed to preview materialized dataset view",
                "warnings": warnings,
                "errors": errors,
            })),
        )
            .into_response();
    }

    match preview_ad_hoc_view(&state, &view, limit, offset).await {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": error }))).into_response(),
    }
}

async fn preview_ad_hoc_view(
    state: &AppState,
    view: &DatasetView,
    limit: i64,
    offset: i64,
) -> Result<serde_json::Value, String> {
    let source = runtime::resolve_dataset_source(
        state,
        view.dataset_id,
        view.source_branch.as_deref(),
        view.source_version,
    )
    .await
    .map_err(|error| match error {
        runtime::DatasetSourceError::Invalid(message)
        | runtime::DatasetSourceError::Database(message) => message,
    })?
    .ok_or_else(|| "source dataset not found for view".to_string())?;

    let bytes = state
        .storage
        .get(&source.storage_path)
        .await
        .map_err(|error| error.to_string())?;
    let prepared = runtime::prepare_query_context(&source.dataset.format, &bytes).await?;
    let wrapped = runtime::wrap_query(&view.sql_text);
    let columns = runtime::load_schema_fields_for_query(&prepared.ctx, &wrapped).await?;
    let total_rows =
        runtime::fetch_scalar_i64(&prepared.ctx, &runtime::count_query(&view.sql_text)).await?;
    let rows = runtime::collect_object_rows(
        &prepared.ctx,
        &runtime::paged_query(&view.sql_text, limit, offset),
    )
    .await?;
    runtime::cleanup_temp_path(prepared.path).await;

    Ok(json!({
        "view_id": view.id,
        "view_name": view.name,
        "materialized": false,
        "dataset_id": view.dataset_id,
        "source_branch": source.branch,
        "source_version": source.version,
        "current_version": 0,
        "row_count": total_rows,
        "columns": columns,
        "rows": rows,
        "limit": limit,
        "offset": offset,
    }))
}

pub async fn refresh_view_materialization(
    state: &AppState,
    dataset: &Dataset,
    view: &DatasetView,
) -> Result<DatasetView, String> {
    let source = runtime::resolve_dataset_source(
        state,
        view.dataset_id,
        view.source_branch.as_deref(),
        view.source_version,
    )
    .await
    .map_err(|error| match error {
        runtime::DatasetSourceError::Invalid(message)
        | runtime::DatasetSourceError::Database(message) => message,
    })?
    .ok_or_else(|| "source dataset not found for view".to_string())?;

    let bytes = state
        .storage
        .get(&source.storage_path)
        .await
        .map_err(|error| error.to_string())?;
    let prepared = runtime::prepare_query_context(&source.dataset.format, &bytes).await?;
    let query = runtime::wrap_query(&view.sql_text);
    let columns = runtime::load_schema_fields_for_query(&prepared.ctx, &query).await?;
    let rows = runtime::collect_object_rows(&prepared.ctx, &query).await?;
    runtime::cleanup_temp_path(prepared.path).await;

    let next_version = view.current_version + 1;
    let storage_path = format!(
        "{}/views/{}/v{}.json",
        dataset.storage_path, view.id, next_version
    );
    let payload = runtime::json_bytes(&rows)?;
    state
        .storage
        .put(&storage_path, Bytes::from(payload.clone()))
        .await
        .map_err(|error| error.to_string())?;

    let mut tx = state.db.begin().await.map_err(|error| error.to_string())?;
    lock_dataset(&mut tx, dataset.id)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::query_as::<_, DatasetView>(
        r#"UPDATE dataset_views
           SET current_version = $3,
               storage_path = $4,
               row_count = $5,
               schema_fields = $6::jsonb,
               last_refreshed_at = NOW(),
               updated_at = NOW()
           WHERE id = $1 AND dataset_id = $2
           RETURNING *"#,
    )
    .bind(view.id)
    .bind(dataset.id)
    .bind(next_version)
    .bind(&storage_path)
    .bind(rows.len() as i64)
    .bind(runtime::schema_to_value(&columns).map_err(|error| error.to_string())?)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;

    let _ = transactions::record_committed_transaction(
        &mut tx,
        dataset.id,
        transactions::TransactionRecord {
            view_id: Some(view.id),
            operation: "refresh_view".to_string(),
            branch_name: source.branch.clone(),
            summary: format!("Refreshed dataset view '{}'", view.name),
            metadata: json!({
                "view_name": view.name,
                "source_version": source.version,
                "view_version": next_version,
                "row_count": rows.len(),
                "storage_path": storage_path,
            }),
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    if let Err(error) = tx.commit().await {
        let _ = state.storage.delete(&storage_path).await;
        return Err(error.to_string());
    }

    load_view(state, dataset.id, view.id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "dataset view disappeared after refresh".to_string())
}

pub async fn load_view(
    state: &AppState,
    dataset_id: Uuid,
    view_id: Uuid,
) -> Result<Option<DatasetView>, sqlx::Error> {
    sqlx::query_as::<_, DatasetView>(
        "SELECT * FROM dataset_views WHERE dataset_id = $1 AND id = $2",
    )
    .bind(dataset_id)
    .bind(view_id)
    .fetch_optional(&state.db)
    .await
}

async fn load_dataset(state: &AppState, dataset_id: Uuid) -> Result<Option<Dataset>, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
}

async fn lock_dataset(
    tx: &mut Transaction<'_, Postgres>,
    dataset_id: Uuid,
) -> Result<Dataset, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1 FOR UPDATE")
        .bind(dataset_id)
        .fetch_one(&mut **tx)
        .await
}
