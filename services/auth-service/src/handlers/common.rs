use crate::domain::rbac;
use crate::models::user::{User, UserResponse};
use auth_middleware::Claims;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use sqlx::PgPool;

pub fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

pub fn require_permission(claims: &Claims, resource: &str, action: &str) -> Result<(), Response> {
    if claims.has_permission(resource, action) {
        Ok(())
    } else {
        Err(json_error(
            StatusCode::FORBIDDEN,
            format!("missing permission {resource}:{action}"),
        ))
    }
}

pub async fn build_user_response(pool: &PgPool, user: User) -> Result<UserResponse, sqlx::Error> {
    let access_bundle = rbac::get_user_access_bundle(pool, user.id)
        .await
        .unwrap_or_default();
    let mfa_enabled = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_mfa_totp WHERE user_id = $1 AND enabled = true)",
    )
    .bind(user.id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let mut response = user.into_response(access_bundle.roles);
    response.groups = access_bundle.groups;
    response.permissions = access_bundle.permissions;
    response.mfa_enabled = mfa_enabled;
    Ok(response)
}
