use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::models::object_type::*;
use crate::AppState;
use auth_middleware::layer::AuthUser;

pub async fn create_object_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateObjectTypeRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let description = body.description.unwrap_or_default();

    let result = sqlx::query_as::<_, ObjectType>(
        r#"INSERT INTO object_types (id, name, display_name, description, primary_key_property, icon, color, owner_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&display_name)
    .bind(&description)
    .bind(&body.primary_key_property)
    .bind(&body.icon)
    .bind(&body.color)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(ot) => (StatusCode::CREATED, Json(serde_json::json!(ot))).into_response(),
        Err(e) => {
            tracing::error!("create object type: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_object_types(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListObjectTypesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let search_pattern = format!("%{search}%");

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM object_types WHERE name ILIKE $1 OR display_name ILIKE $1",
    )
    .bind(&search_pattern)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let types = sqlx::query_as::<_, ObjectType>(
        r#"SELECT * FROM object_types
           WHERE name ILIKE $1 OR display_name ILIKE $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(ListObjectTypesResponse { data: types, total, page, per_page })
}

pub async fn get_object_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(ot)) => Json(serde_json::json!(ot)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_object_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateObjectTypeRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ObjectType>(
        r#"UPDATE object_types SET
           display_name = COALESCE($2, display_name),
           description = COALESCE($3, description),
           primary_key_property = COALESCE($4, primary_key_property),
           icon = COALESCE($5, icon),
           color = COALESCE($6, color),
           updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.display_name)
    .bind(&body.description)
    .bind(&body.primary_key_property)
    .bind(&body.icon)
    .bind(&body.color)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(ot)) => Json(serde_json::json!(ot)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_object_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM object_types WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
