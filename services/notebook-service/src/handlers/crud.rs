use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::cell::{Cell, CreateCellRequest, UpdateCellRequest};
use crate::models::notebook::*;
use auth_middleware::layer::AuthUser;

// ── Notebook CRUD ──

pub async fn create_notebook(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateNotebookRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let description = body.description.unwrap_or_default();
    let kernel = body.default_kernel.unwrap_or_else(|| "python".to_string());

    let result = sqlx::query_as::<_, Notebook>(
        r#"INSERT INTO notebooks (id, name, description, owner_id, default_kernel)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&description)
    .bind(claims.sub)
    .bind(&kernel)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(nb) => (StatusCode::CREATED, Json(serde_json::json!(nb))).into_response(),
        Err(e) => {
            tracing::error!("create notebook: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_notebooks(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListNotebooksQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let pattern = format!("%{search}%");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notebooks WHERE name ILIKE $1")
        .bind(&pattern)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let notebooks = sqlx::query_as::<_, Notebook>(
        r#"SELECT * FROM notebooks WHERE name ILIKE $1
           ORDER BY updated_at DESC LIMIT $2 OFFSET $3"#,
    )
    .bind(&pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(
        serde_json::json!({ "data": notebooks, "total": total, "page": page, "per_page": per_page }),
    )
}

pub async fn get_notebook(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let notebook = sqlx::query_as::<_, Notebook>("SELECT * FROM notebooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await;

    let cells = sqlx::query_as::<_, Cell>(
        "SELECT * FROM cells WHERE notebook_id = $1 ORDER BY position ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    match notebook {
        Ok(Some(nb)) => Json(serde_json::json!({ "notebook": nb, "cells": cells })).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_notebook(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNotebookRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Notebook>(
        r#"UPDATE notebooks SET
           name = COALESCE($2, name),
           description = COALESCE($3, description),
           default_kernel = COALESCE($4, default_kernel),
           updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.default_kernel)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(nb)) => Json(serde_json::json!(nb)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_notebook(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM notebooks WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Cell CRUD ──

pub async fn add_cell(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
    Json(body): Json<CreateCellRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let cell_type = body.cell_type.unwrap_or_else(|| "code".to_string());
    let kernel = body.kernel.unwrap_or_else(|| "python".to_string());
    let source = body.source.unwrap_or_default();

    // Get next position
    let max_pos: Option<i32> =
        sqlx::query_scalar("SELECT MAX(position) FROM cells WHERE notebook_id = $1")
            .bind(notebook_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(None);

    let position = body.position.unwrap_or_else(|| max_pos.unwrap_or(0) + 1);

    let result = sqlx::query_as::<_, Cell>(
        r#"INSERT INTO cells (id, notebook_id, cell_type, kernel, source, position)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *"#,
    )
    .bind(id)
    .bind(notebook_id)
    .bind(&cell_type)
    .bind(&kernel)
    .bind(&source)
    .bind(position)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(cell) => (StatusCode::CREATED, Json(serde_json::json!(cell))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_cell(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_notebook_id, cell_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCellRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Cell>(
        r#"UPDATE cells SET
           source = COALESCE($2, source),
           cell_type = COALESCE($3, cell_type),
           kernel = COALESCE($4, kernel),
           position = COALESCE($5, position),
           updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(cell_id)
    .bind(&body.source)
    .bind(&body.cell_type)
    .bind(&body.kernel)
    .bind(body.position)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(c)) => Json(serde_json::json!(c)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_cell(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_notebook_id, cell_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM cells WHERE id = $1")
        .bind(cell_id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
