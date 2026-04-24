use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::models::link_type::*;
use auth_middleware::layer::AuthUser;

// --- Link Type CRUD ---

pub async fn create_link_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateLinkTypeRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let description = body.description.unwrap_or_default();
    let cardinality = body
        .cardinality
        .unwrap_or_else(|| "many_to_many".to_string());

    let result = sqlx::query_as::<_, LinkType>(
        r#"INSERT INTO link_types (id, name, display_name, description, source_type_id, target_type_id, cardinality, owner_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&display_name)
    .bind(&description)
    .bind(body.source_type_id)
    .bind(body.target_type_id)
    .bind(&cardinality)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(lt) => (StatusCode::CREATED, Json(serde_json::json!(lt))).into_response(),
        Err(e) => {
            tracing::error!("create link type: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_link_types(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListLinkTypesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let (types, total) = if let Some(ot_id) = params.object_type_id {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM link_types WHERE source_type_id = $1 OR target_type_id = $1",
        )
        .bind(ot_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

        let types = sqlx::query_as::<_, LinkType>(
            r#"SELECT * FROM link_types
               WHERE source_type_id = $1 OR target_type_id = $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(ot_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        (types, total)
    } else {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM link_types")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        let types = sqlx::query_as::<_, LinkType>(
            "SELECT * FROM link_types ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        (types, total)
    };

    Json(serde_json::json!({ "data": types, "total": total, "page": page, "per_page": per_page }))
}

pub async fn delete_link_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM link_types WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- Link Instance CRUD ---

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct LinkInstance {
    pub id: Uuid,
    pub link_type_id: Uuid,
    pub source_object_id: Uuid,
    pub target_object_id: Uuid,
    pub properties: Option<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkRequest {
    pub source_object_id: Uuid,
    pub target_object_id: Uuid,
    pub properties: Option<serde_json::Value>,
}

pub async fn create_link(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(link_type_id): Path<Uuid>,
    Json(body): Json<CreateLinkRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, LinkInstance>(
        r#"INSERT INTO link_instances (id, link_type_id, source_object_id, target_object_id, properties, created_by)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *"#,
    )
    .bind(id)
    .bind(link_type_id)
    .bind(body.source_object_id)
    .bind(body.target_object_id)
    .bind(&body.properties)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(link) => (StatusCode::CREATED, Json(serde_json::json!(link))).into_response(),
        Err(e) => {
            tracing::error!("create link: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_links(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(link_type_id): Path<Uuid>,
    Query(params): Query<ListLinkTypesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let links = sqlx::query_as::<_, LinkInstance>(
        r#"SELECT * FROM link_instances
           WHERE link_type_id = $1
           ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
    )
    .bind(link_type_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "data": links }))
}

pub async fn delete_link(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_link_type_id, link_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM link_instances WHERE id = $1")
        .bind(link_id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
