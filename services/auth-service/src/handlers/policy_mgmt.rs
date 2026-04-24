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
use crate::domain::abac;
use crate::models::policy::Policy;

use super::common::{json_error, require_permission};

#[derive(Debug, Deserialize)]
pub struct UpsertPolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub effect: String,
    pub resource: String,
    pub action: String,
    #[serde(default)]
    pub conditions: Value,
    pub row_filter: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct EvaluatePolicyRequest {
    pub resource: String,
    pub action: String,
    #[serde(default)]
    pub resource_attributes: Value,
}

pub async fn list_policies(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "policies", "read") {
        return response;
    }

    match abac::list_policies(&state.db).await {
        Ok(policies) => Json(policies).into_response(),
        Err(e) => {
            tracing::error!("failed to list policies: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_policy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpsertPolicyRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "policies", "write") {
        return response;
    }

    match sqlx::query_as::<_, Policy>(
        r#"INSERT INTO abac_policies (id, name, description, effect, resource, action, conditions, row_filter, enabled, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, description, effect, resource, action, conditions, row_filter, enabled, created_by, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name)
    .bind(body.description)
    .bind(body.effect)
    .bind(body.resource)
    .bind(body.action)
    .bind(body.conditions)
    .bind(body.row_filter)
    .bind(body.enabled)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(policy) => (StatusCode::CREATED, Json(policy)).into_response(),
        Err(e) => {
            tracing::error!("failed to create policy: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create policy")
        }
    }
}

pub async fn update_policy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(policy_id): Path<Uuid>,
    Json(body): Json<UpsertPolicyRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "policies", "write") {
        return response;
    }

    match sqlx::query_as::<_, Policy>(
        r#"UPDATE abac_policies
           SET name = $2,
               description = $3,
               effect = $4,
               resource = $5,
               action = $6,
               conditions = $7,
               row_filter = $8,
               enabled = $9,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, description, effect, resource, action, conditions, row_filter, enabled, created_by, created_at, updated_at"#,
    )
    .bind(policy_id)
    .bind(body.name)
    .bind(body.description)
    .bind(body.effect)
    .bind(body.resource)
    .bind(body.action)
    .bind(body.conditions)
    .bind(body.row_filter)
    .bind(body.enabled)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(policy)) => Json(policy).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to update policy: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_policy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(policy_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "policies", "write") {
        return response;
    }

    match sqlx::query("DELETE FROM abac_policies WHERE id = $1")
        .bind(policy_id)
        .execute(&state.db)
        .await
    {
        Ok(record) if record.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to delete policy: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn evaluate_policy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<EvaluatePolicyRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "policies", "read") {
        return response;
    }

    match abac::evaluate(
        &state.db,
        &claims,
        &body.resource,
        &body.action,
        &body.resource_attributes,
    )
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            tracing::error!("failed to evaluate ABAC policies: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
