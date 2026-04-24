use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use auth_middleware::layer::AuthUser;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ObjectInstance {
    pub id: Uuid,
    pub object_type_id: Uuid,
    pub properties: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateObjectRequest {
    pub properties: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn create_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<CreateObjectRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, ObjectInstance>(
        r#"INSERT INTO object_instances (id, object_type_id, properties, created_by)
           VALUES ($1, $2, $3, $4)
           RETURNING *"#,
    )
    .bind(id)
    .bind(type_id)
    .bind(&body.properties)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(obj) => (StatusCode::CREATED, Json(serde_json::json!(obj))).into_response(),
        Err(e) => {
            tracing::error!("create object: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

pub async fn list_objects(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Query(params): Query<ListObjectsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM object_instances WHERE object_type_id = $1")
            .bind(type_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let objects = sqlx::query_as::<_, ObjectInstance>(
        r#"SELECT * FROM object_instances
           WHERE object_type_id = $1
           ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
    )
    .bind(type_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "data": objects, "total": total, "page": page, "per_page": per_page }))
}

pub async fn get_object(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, ObjectInstance>("SELECT * FROM object_instances WHERE id = $1")
        .bind(obj_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(obj)) => Json(serde_json::json!(obj)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_object(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM object_instances WHERE id = $1")
        .bind(obj_id)
        .execute(&state.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
