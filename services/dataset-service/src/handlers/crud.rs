use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::dataset::{
    CreateDatasetRequest, Dataset, ListDatasetsQuery, UpdateDatasetRequest,
};

/// POST /api/v1/datasets
pub async fn create_dataset(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<CreateDatasetRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let format = body.format.unwrap_or_else(|| "parquet".to_string());
    let storage_path = format!("datasets/{id}");
    let tags = body.tags.unwrap_or_default();

    let result = sqlx::query_as::<_, Dataset>(
          r#"INSERT INTO datasets (id, name, description, format, storage_path, owner_id, tags, active_branch)
              VALUES ($1, $2, $3, $4, $5, $6, $7, 'main')
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(body.description.as_deref().unwrap_or(""))
    .bind(&format)
    .bind(&storage_path)
    .bind(claims.sub)
    .bind(&tags)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(ds) => {
            let _ = sqlx::query(
                r#"INSERT INTO dataset_branches (
                       id, dataset_id, name, version, base_version, description, is_default
                   )
                   VALUES ($1, $2, 'main', $3, $3, 'Default branch', TRUE)
                   ON CONFLICT (dataset_id, name) DO NOTHING"#,
            )
            .bind(Uuid::now_v7())
            .bind(ds.id)
            .bind(ds.current_version)
            .execute(&state.db)
            .await;

            (StatusCode::CREATED, Json(ds)).into_response()
        }
        Err(e) => {
            tracing::error!("create dataset failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "create failed" })),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/datasets
pub async fn list_datasets(
    State(state): State<AppState>,
    Query(params): Query<ListDatasetsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let search_pattern = params.search.map(|s| format!("%{s}%"));

    let datasets = sqlx::query_as::<_, Dataset>(
        r#"SELECT * FROM datasets
           WHERE ($1::TEXT IS NULL OR name ILIKE $1 OR description ILIKE $1)
             AND ($2::TEXT IS NULL OR $2 = ANY(tags))
                         AND ($3::UUID IS NULL OR owner_id = $3)
                     ORDER BY created_at DESC
                     LIMIT $4 OFFSET $5"#,
    )
    .bind(&search_pattern)
    .bind(&params.tag)
    .bind(params.owner_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await;

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM datasets
           WHERE ($1::TEXT IS NULL OR name ILIKE $1 OR description ILIKE $1)
                         AND ($2::TEXT IS NULL OR $2 = ANY(tags))
                         AND ($3::UUID IS NULL OR owner_id = $3)"#,
    )
    .bind(&search_pattern)
    .bind(&params.tag)
    .bind(params.owner_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    match datasets {
        Ok(ds) => Json(serde_json::json!({
            "data": ds,
            "page": page,
            "per_page": per_page,
            "total": total,
            "total_pages": (total as f64 / per_page as f64).ceil() as i64,
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("list datasets failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// GET /api/v1/datasets/:id
pub async fn get_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let ds = sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await;

    match ds {
        Ok(Some(d)) => Json(d).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get dataset failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// PATCH /api/v1/datasets/:id
pub async fn update_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(body): Json<UpdateDatasetRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Dataset>(
        r#"UPDATE datasets
           SET name = COALESCE($2, name),
               description = COALESCE($3, description),
               tags = COALESCE($4, tags),
               owner_id = COALESCE($5, owner_id),
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(dataset_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.tags)
    .bind(body.owner_id)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(d)) => Json(d).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("update dataset failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// DELETE /api/v1/datasets/:id
pub async fn delete_dataset(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    // Delete storage files
    if let Ok(Some(ds)) = sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        let _ = state.storage.delete(&ds.storage_path).await;
    }

    let result = sqlx::query("DELETE FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("delete dataset failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
