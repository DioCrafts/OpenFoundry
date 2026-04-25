use auth_middleware::{claims::SessionScope, layer::AuthUser};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::sessions::{self, ScopedSessionError},
    models::{
        session::{
            CreateGuestSessionRequest, CreateScopedSessionRequest, ScopedSessionKind,
        },
        user::User,
    },
};

use super::common::json_error;

pub async fn list_scoped_sessions(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if !claims.has_permission("sessions", "self") && !claims.has_permission("sessions", "write") {
        return json_error(
            StatusCode::FORBIDDEN,
            "missing permission sessions:self",
        );
    }

    match sessions::list_scoped_sessions(&state.db, claims.sub).await {
        Ok(items) => Json(items).into_response(),
        Err(error) => {
            tracing::error!("failed to list scoped sessions: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_scoped_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateScopedSessionRequest>,
) -> impl IntoResponse {
    if !claims.has_permission("sessions", "self") && !claims.has_permission("sessions", "write") {
        return json_error(
            StatusCode::FORBIDDEN,
            "missing permission sessions:self",
        );
    }

    if body.label.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "label is required");
    }
    if let Err(message) = validate_requested_permissions(&claims, &body.permissions) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_clearance(&claims, body.classification_clearance.as_deref()) {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    let user = match load_current_user(&state.db, claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("failed to load current user for scoped session: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let scope = SessionScope {
        allowed_methods: body.allowed_methods,
        allowed_path_prefixes: body.allowed_path_prefixes,
        allowed_subject_ids: body.allowed_subject_ids,
        allowed_org_ids: body.allowed_org_ids,
        workspace: body.workspace,
        classification_clearance: body.classification_clearance,
        guest_email: None,
        guest_display_name: None,
    };

    match sessions::issue_scoped_session(
        &state.db,
        &state.jwt_config,
        &user,
        body.label.trim(),
        ScopedSessionKind::Scoped,
        body.permissions,
        scope,
        body.expires_at,
        None,
        None,
    )
    .await
    {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(ScopedSessionError::InvalidExpiration) => {
            json_error(StatusCode::BAD_REQUEST, "expires_at must be in the future")
        }
        Err(ScopedSessionError::Database(error)) => {
            tracing::error!("failed to persist scoped session: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(ScopedSessionError::Token(error)) => {
            tracing::error!("failed to issue scoped session token: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_guest_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateGuestSessionRequest>,
) -> impl IntoResponse {
    if !claims.has_permission("guests", "write") && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission guests:write");
    }

    if body.label.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "label is required");
    }
    if body.guest_email.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "guest_email is required");
    }
    if let Err(message) = validate_requested_permissions(&claims, &body.permissions) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if body
        .permissions
        .iter()
        .any(|permission| !permission_is_guest_safe(permission))
    {
        return json_error(
            StatusCode::FORBIDDEN,
            "guest sessions can only issue read/self style permissions",
        );
    }
    if let Err(message) =
        validate_requested_clearance(&claims, body.classification_clearance.as_deref())
    {
        return json_error(StatusCode::FORBIDDEN, message);
    }

    let user = match load_current_user(&state.db, claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("failed to load current user for guest session: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let scope = SessionScope {
        allowed_methods: body.allowed_methods,
        allowed_path_prefixes: body.allowed_path_prefixes,
        allowed_subject_ids: body.allowed_subject_ids,
        allowed_org_ids: body.allowed_org_ids,
        workspace: body.workspace,
        classification_clearance: body.classification_clearance,
        guest_email: Some(body.guest_email.clone()),
        guest_display_name: body.guest_name.clone(),
    };

    match sessions::issue_scoped_session(
        &state.db,
        &state.jwt_config,
        &user,
        body.label.trim(),
        ScopedSessionKind::Guest,
        body.permissions,
        scope,
        body.expires_at,
        Some(body.guest_email),
        body.guest_name,
    )
    .await
    {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(ScopedSessionError::InvalidExpiration) => {
            json_error(StatusCode::BAD_REQUEST, "expires_at must be in the future")
        }
        Err(ScopedSessionError::Database(error)) => {
            tracing::error!("failed to persist guest session: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(ScopedSessionError::Token(error)) => {
            tracing::error!("failed to issue guest session token: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn revoke_scoped_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    if !claims.has_permission("sessions", "self") && !claims.has_permission("sessions", "write") {
        return json_error(
            StatusCode::FORBIDDEN,
            "missing permission sessions:self",
        );
    }

    match sessions::revoke_scoped_session(
        &state.db,
        session_id,
        claims.sub,
        claims.has_permission("sessions", "write"),
    )
    .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("failed to revoke scoped session: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn validate_requested_permissions(
    claims: &auth_middleware::Claims,
    requested_permissions: &[String],
) -> Result<(), String> {
    if claims.has_permission("sessions", "write") || requested_permissions.is_empty() {
        return Ok(());
    }

    if requested_permissions
        .iter()
        .all(|permission| claims.has_permission_key(permission))
    {
        Ok(())
    } else {
        Err("requested permissions exceed caller permissions".to_string())
    }
}

fn validate_requested_clearance(
    claims: &auth_middleware::Claims,
    requested_clearance: Option<&str>,
) -> Result<(), String> {
    let Some(requested_clearance) = requested_clearance else {
        return Ok(());
    };
    let requested_rank = clearance_rank(requested_clearance)
        .ok_or_else(|| "unsupported classification_clearance".to_string())?;
    let caller_rank = claims
        .classification_clearance()
        .and_then(clearance_rank)
        .unwrap_or_else(|| {
            if claims.has_role("admin") {
                2
            } else {
                0
            }
        });

    if claims.has_role("admin") || requested_rank <= caller_rank {
        Ok(())
    } else {
        Err("requested classification_clearance exceeds caller clearance".to_string())
    }
}

fn clearance_rank(value: &str) -> Option<u8> {
    match value {
        "public" => Some(0),
        "confidential" => Some(1),
        "pii" => Some(2),
        _ => None,
    }
}

fn permission_is_guest_safe(permission: &str) -> bool {
    permission.ends_with(":read") || permission.ends_with(":self") || permission == "*:read"
}

async fn load_current_user(pool: &sqlx::PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1 AND is_active = true",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn claims_with_clearance(clearance: &str) -> auth_middleware::Claims {
        auth_middleware::Claims {
            sub: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            jti: Uuid::nil(),
            email: "user@example.com".to_string(),
            name: "User".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: vec!["datasets:read".to_string(), "lineage:read".to_string()],
            org_id: Some(Uuid::nil()),
            attributes: json!({ "classification_clearance": clearance }),
            auth_methods: vec!["password".to_string()],
            token_use: Some("access".to_string()),
            api_key_id: None,
            session_kind: None,
            session_scope: None,
        }
    }

    #[test]
    fn guest_permissions_are_restricted() {
        assert!(permission_is_guest_safe("datasets:read"));
        assert!(!permission_is_guest_safe("datasets:write"));
    }

    #[test]
    fn requested_clearance_cannot_exceed_caller() {
        let claims = claims_with_clearance("confidential");
        assert!(validate_requested_clearance(&claims, Some("public")).is_ok());
        assert!(validate_requested_clearance(&claims, Some("pii")).is_err());
    }
}
