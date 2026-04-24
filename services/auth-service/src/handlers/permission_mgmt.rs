use auth_middleware::layer::AuthUser;
use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;

use crate::AppState;
use crate::models::permission::Permission;

use super::common::{json_error, require_permission};

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
}

pub async fn list_permissions(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "permissions", "read") {
        return response;
    }

    match sqlx::query_as::<_, Permission>(
        "SELECT id, resource, action, description, created_at FROM permissions ORDER BY resource, action",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(permissions) => Json(permissions).into_response(),
        Err(e) => {
            tracing::error!("failed to list permissions: {e}");
            json_error(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "failed to list permissions")
        }
    }
}

pub async fn create_permission(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreatePermissionRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "permissions", "write") {
        return response;
    }

    match sqlx::query_as::<_, Permission>(
        r#"INSERT INTO permissions (id, resource, action, description)
           VALUES ($1, $2, $3, $4)
           RETURNING id, resource, action, description, created_at"#,
    )
    .bind(uuid::Uuid::now_v7())
    .bind(body.resource)
    .bind(body.action)
    .bind(body.description)
    .fetch_one(&state.db)
    .await
    {
        Ok(permission) => (axum::http::StatusCode::CREATED, Json(permission)).into_response(),
        Err(e) => {
            tracing::error!("failed to create permission: {e}");
            json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create permission",
            )
        }
    }
}
