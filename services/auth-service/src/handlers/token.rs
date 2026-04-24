use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppState;
use crate::domain::jwt::{
    get_refresh_token, issue_tokens, refresh_token_matches, revoke_refresh_token,
};
use crate::handlers::login::TokenResponse;
use crate::models::user::User;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> impl IntoResponse {
    // Decode the refresh token
    let claims = match auth_middleware::jwt::decode_token(&state.jwt_config, &body.refresh_token) {
        Ok(c) if c.token_use.as_deref() == Some("refresh") => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid refresh token" })),
            )
                .into_response();
        }
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid refresh token" })),
            )
                .into_response();
        }
    };

    let Some(stored_token) = (match get_refresh_token(&state.db, claims.jti).await {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("failed to load refresh token: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "token refresh failed" })),
            )
                .into_response();
        }
    }) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "refresh token revoked" })),
        )
            .into_response();
    };

    if !refresh_token_matches(&stored_token, &body.refresh_token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "refresh token revoked" })),
        )
            .into_response();
    }

    // Fetch the user
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1 AND is_active = true",
    )
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "user not found or disabled" })),
            )
                .into_response();
        }
    };

    if let Err(e) = revoke_refresh_token(&state.db, claims.jti).await {
        tracing::error!("failed to revoke refresh token: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "token refresh failed" })),
        )
            .into_response();
    }

    match issue_tokens(
        &state.db,
        &state.jwt_config,
        &user,
        vec!["refresh".to_string()],
    )
    .await
    {
        Ok((access_token, refresh_token)) => Json(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_config.access_ttl_secs,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("token refresh failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "token refresh failed" })),
            )
                .into_response()
        }
    }
}
