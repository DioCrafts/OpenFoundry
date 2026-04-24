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
use crate::models::group::Group;
use crate::models::role::Role;

use super::common::{json_error, require_permission};

#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub member_count: i64,
    pub role_ids: Vec<Uuid>,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub description: Option<String>,
    #[serde(default)]
    pub role_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UserGroupRequest {
    pub group_id: Uuid,
}

pub async fn list_groups(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "groups", "read") {
        return response;
    }

    match sqlx::query_as::<_, Group>(
        "SELECT id, name, description, created_at FROM groups ORDER BY name",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(groups) => {
            let mut responses = Vec::with_capacity(groups.len());
            for group in groups {
                match build_group_response(&state.db, group).await {
                    Ok(response) => responses.push(response),
                    Err(e) => {
                        tracing::error!("failed to build group response: {e}");
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                }
            }
            Json(responses).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list groups: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_group(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "groups", "write") {
        return response;
    }

    let group_id = Uuid::now_v7();
    match sqlx::query("INSERT INTO groups (id, name, description) VALUES ($1, $2, $3)")
        .bind(group_id)
        .bind(&body.name)
        .bind(&body.description)
        .execute(&state.db)
        .await
    {
        Ok(_) => {
            if let Err(e) = replace_group_roles(&state.db, group_id, &body.role_ids).await {
                tracing::error!("failed to assign group roles: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            match sqlx::query_as::<_, Group>(
                "SELECT id, name, description, created_at FROM groups WHERE id = $1",
            )
            .bind(group_id)
            .fetch_one(&state.db)
            .await
            {
                Ok(group) => match build_group_response(&state.db, group).await {
                    Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
                    Err(e) => {
                        tracing::error!("failed to build created group response: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
                Err(e) => {
                    tracing::error!("failed to fetch created group: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to create group: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create group")
        }
    }
}

pub async fn update_group(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(group_id): Path<Uuid>,
    Json(body): Json<UpdateGroupRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "groups", "write") {
        return response;
    }

    match sqlx::query("UPDATE groups SET description = $2 WHERE id = $1")
        .bind(group_id)
        .bind(body.description)
        .execute(&state.db)
        .await
    {
        Ok(record) if record.rows_affected() == 0 => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => {
            if let Err(e) = replace_group_roles(&state.db, group_id, &body.role_ids).await {
                tracing::error!("failed to replace group roles: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }

            match sqlx::query_as::<_, Group>(
                "SELECT id, name, description, created_at FROM groups WHERE id = $1",
            )
            .bind(group_id)
            .fetch_one(&state.db)
            .await
            {
                Ok(group) => match build_group_response(&state.db, group).await {
                    Ok(response) => Json(response).into_response(),
                    Err(e) => {
                        tracing::error!("failed to build updated group response: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                },
                Err(e) => {
                    tracing::error!("failed to fetch updated group: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to update group: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn add_user_to_group(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserGroupRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "groups", "write") {
        return response;
    }

    match sqlx::query(
        "INSERT INTO group_members (group_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(body.group_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("failed to add user to group: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn remove_user_from_group(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((user_id, group_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "groups", "write") {
        return response;
    }

    match sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(user_id)
        .execute(&state.db)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("failed to remove user from group: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn build_group_response(
    pool: &sqlx::PgPool,
    group: Group,
) -> Result<GroupResponse, sqlx::Error> {
    let roles = sqlx::query_as::<_, Role>(
        r#"SELECT r.id, r.name, r.description, r.created_at
           FROM roles r
           INNER JOIN group_roles gr ON gr.role_id = r.id
           WHERE gr.group_id = $1
           ORDER BY r.name"#,
    )
    .bind(group.id)
    .fetch_all(pool)
    .await?;

    let member_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM group_members WHERE group_id = $1",
    )
    .bind(group.id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(GroupResponse {
        id: group.id,
        name: group.name,
        description: group.description,
        created_at: group.created_at,
        member_count,
        role_ids: roles.iter().map(|role| role.id).collect(),
        roles: roles.into_iter().map(|role| role.name).collect(),
    })
}

async fn replace_group_roles(
    pool: &sqlx::PgPool,
    group_id: Uuid,
    role_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM group_roles WHERE group_id = $1")
        .bind(group_id)
        .execute(&mut *transaction)
        .await?;

    for role_id in role_ids {
        sqlx::query("INSERT INTO group_roles (group_id, role_id) VALUES ($1, $2)")
            .bind(group_id)
            .bind(role_id)
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await
}
