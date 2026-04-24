use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::domain::api_keys::{self, ApiKeyError};
use crate::models::user::User;

use super::common::json_error;

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if !claims.has_permission("api_keys", "self") && !claims.has_permission("api_keys", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission api_keys:self");
    }

    match api_keys::list_api_keys(&state.db, claims.sub).await {
        Ok(api_keys) => Json(api_keys).into_response(),
        Err(e) => {
            tracing::error!("failed to list API keys: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_api_key(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    if !claims.has_permission("api_keys", "self") && !claims.has_permission("api_keys", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission api_keys:self");
    }

    if !claims.has_permission("api_keys", "write")
        && body
            .scopes
            .iter()
            .any(|scope| !claims.has_permission_key(scope))
    {
        return json_error(
            StatusCode::FORBIDDEN,
            "requested scopes exceed caller permissions",
        );
    }

    let user = match sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("failed to load current user for API key creation: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match api_keys::create_api_key(
        &state.db,
        &state.jwt_config,
        &user,
        &body.name,
        body.scopes,
        body.expires_at,
    )
    .await
    {
        Ok(api_key) => (StatusCode::CREATED, Json(api_key)).into_response(),
        Err(ApiKeyError::InvalidExpiration) => {
            json_error(StatusCode::BAD_REQUEST, "expires_at must be in the future")
        }
        Err(ApiKeyError::Database(error)) => {
            tracing::error!("failed to persist API key: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(ApiKeyError::Token(error)) => {
            tracing::error!("failed to issue API key token: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(api_key_id): Path<Uuid>,
) -> impl IntoResponse {
    if !claims.has_permission("api_keys", "self") && !claims.has_permission("api_keys", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission api_keys:self");
    }

    match api_keys::revoke_api_key(&state.db, api_key_id, claims.sub).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to revoke API key: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
