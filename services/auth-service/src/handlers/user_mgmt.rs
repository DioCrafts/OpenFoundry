use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::AppState;
use crate::models::user::User;

use super::common::{build_user_response, json_error, require_permission};

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub organization_id: Option<Uuid>,
    pub attributes: Option<Value>,
    pub mfa_enforced: Option<bool>,
    pub is_active: Option<bool>,
}

/// GET /api/v1/users/me
pub async fn me(State(state): State<AppState>, AuthUser(claims): AuthUser) -> impl IntoResponse {
    match load_user(&state.db, claims.sub).await {
        Ok(Some(user)) => match build_user_response(&state.db, user).await {
            Ok(response) => Json(response).into_response(),
            Err(e) => {
                tracing::error!("failed to build user response: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to fetch current user: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// GET /api/v1/users
pub async fn list_users(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "users", "read") {
        return response;
    }

    let users = sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await;

    match users {
        Ok(users) => {
            let mut responses = Vec::with_capacity(users.len());
            for user in users {
                match build_user_response(&state.db, user).await {
                    Ok(response) => responses.push(response),
                    Err(e) => {
                        tracing::error!("failed to build user response: {e}");
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                }
            }

            Json(responses).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list users: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// PATCH /api/v1/users/:id
pub async fn update_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "users", "write") {
        return response;
    }

    let Some(existing_user) = (match load_user(&state.db, user_id).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("failed to load user for update: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let updated_name = body.name.unwrap_or(existing_user.name.clone());
    let updated_organization_id = body.organization_id.or(existing_user.organization_id);
    let updated_attributes = body.attributes.unwrap_or(existing_user.attributes.clone());
    let updated_mfa_enforced = body.mfa_enforced.unwrap_or(existing_user.mfa_enforced);
    let updated_is_active = body.is_active.unwrap_or(existing_user.is_active);

    let result = sqlx::query(
        r#"UPDATE users
           SET name = $2,
               organization_id = $3,
               attributes = $4,
               mfa_enforced = $5,
               is_active = $6,
               updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(user_id)
    .bind(updated_name)
    .bind(updated_organization_id)
    .bind(updated_attributes)
    .bind(updated_mfa_enforced)
    .bind(updated_is_active)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => match load_user(&state.db, user_id).await {
            Ok(Some(user)) => match build_user_response(&state.db, user).await {
                Ok(response) => Json(response).into_response(),
                Err(e) => {
                    tracing::error!("failed to build updated user response: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            },
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                tracing::error!("failed to reload updated user: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => {
            tracing::error!("failed to update user: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to update user")
        }
    }
}

/// DELETE /api/v1/users/:id
pub async fn deactivate_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "users", "write") {
        return response;
    }

    let result =
        sqlx::query("UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&state.db)
            .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to deactivate user: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn load_user(pool: &sqlx::PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
