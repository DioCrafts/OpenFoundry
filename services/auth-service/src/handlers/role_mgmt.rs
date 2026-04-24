use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::domain::rbac;
use crate::models::permission::Permission;
use crate::models::role::Role;

use super::common::{json_error, require_permission};

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub permission_ids: Vec<Uuid>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub permission_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub description: Option<String>,
    #[serde(default)]
    pub permission_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: Uuid,
}

/// GET /api/v1/roles
pub async fn list_roles(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "roles", "read") {
        return response;
    }

    let roles = sqlx::query_as::<_, Role>(
        "SELECT id, name, description, created_at FROM roles ORDER BY name",
    )
    .fetch_all(&state.db)
    .await;

    match roles {
        Ok(roles) => {
            let mut responses = Vec::with_capacity(roles.len());
            for role in roles {
                match build_role_response(&state.db, role).await {
                    Ok(response) => responses.push(response),
                    Err(e) => {
                        tracing::error!("failed to build role response: {e}");
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                }
            }

            Json(responses).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list roles: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /api/v1/roles
pub async fn create_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateRoleRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "roles", "write") {
        return response;
    }

    let role_id = Uuid::now_v7();
    let result = sqlx::query("INSERT INTO roles (id, name, description) VALUES ($1, $2, $3)")
        .bind(role_id)
        .bind(&body.name)
        .bind(&body.description)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            if let Err(e) = replace_role_permissions(&state.db, role_id, &body.permission_ids).await
            {
                tracing::error!("failed to assign role permissions: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            match sqlx::query_as::<_, Role>(
                "SELECT id, name, description, created_at FROM roles WHERE id = $1",
            )
            .bind(role_id)
            .fetch_one(&state.db)
            .await
            {
                Ok(role) => match build_role_response(&state.db, role).await {
                    Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
                    Err(e) => {
                        tracing::error!("failed to build created role response: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
                Err(e) => {
                    tracing::error!("failed to fetch created role: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to create role: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create role")
        }
    }
}

/// PUT /api/v1/roles/:id
pub async fn update_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(role_id): Path<Uuid>,
    Json(body): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "roles", "write") {
        return response;
    }

    let result = sqlx::query("UPDATE roles SET description = $2 WHERE id = $1")
        .bind(role_id)
        .bind(body.description)
        .execute(&state.db)
        .await;

    match result {
        Ok(record) if record.rows_affected() == 0 => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => {
            if let Err(e) = replace_role_permissions(&state.db, role_id, &body.permission_ids).await
            {
                tracing::error!("failed to replace role permissions: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            match sqlx::query_as::<_, Role>(
                "SELECT id, name, description, created_at FROM roles WHERE id = $1",
            )
            .bind(role_id)
            .fetch_one(&state.db)
            .await
            {
                Ok(role) => match build_role_response(&state.db, role).await {
                    Ok(response) => Json(response).into_response(),
                    Err(e) => {
                        tracing::error!("failed to build updated role response: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
                Err(e) => {
                    tracing::error!("failed to fetch updated role: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to update role: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /api/v1/users/:id/roles
pub async fn assign_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AssignRoleRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "roles", "write") {
        return response;
    }

    match rbac::assign_role(&state.db, user_id, body.role_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("role assignment failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// DELETE /api/v1/users/:id/roles/:role_id
pub async fn remove_role(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "roles", "write") {
        return response;
    }

    match rbac::remove_role(&state.db, user_id, role_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("role removal failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn build_role_response(pool: &sqlx::PgPool, role: Role) -> Result<RoleResponse, sqlx::Error> {
    let permissions = sqlx::query_as::<_, Permission>(
        r#"SELECT p.id, p.resource, p.action, p.description, p.created_at
           FROM permissions p
           INNER JOIN role_permissions rp ON rp.permission_id = p.id
           WHERE rp.role_id = $1
           ORDER BY p.resource, p.action"#,
    )
    .bind(role.id)
    .fetch_all(pool)
    .await?;

    Ok(RoleResponse {
        id: role.id,
        name: role.name,
        description: role.description,
        created_at: role.created_at,
        permission_ids: permissions.iter().map(|permission| permission.id).collect(),
        permissions: permissions
            .into_iter()
            .map(|permission| format!("{}:{}", permission.resource, permission.action))
            .collect(),
    })
}

async fn replace_role_permissions(
    pool: &sqlx::PgPool,
    role_id: Uuid,
    permission_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
        .bind(role_id)
        .execute(&mut *transaction)
        .await?;

    for permission_id in permission_ids {
        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2)")
            .bind(role_id)
            .bind(permission_id)
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await
}
