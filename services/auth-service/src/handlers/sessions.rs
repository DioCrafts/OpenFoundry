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
    domain::{
        access,
        sessions::{self, ScopedSessionError},
    },
    models::{
        restricted_view::{RestrictedView, RestrictedViewRow},
        session::{CreateGuestSessionRequest, CreateScopedSessionRequest, ScopedSessionKind},
        user::User,
    },
};

use super::common::json_error;

pub async fn list_scoped_sessions(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if !claims.has_permission("sessions", "self") && !claims.has_permission("sessions", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission sessions:self");
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
        return json_error(StatusCode::FORBIDDEN, "missing permission sessions:self");
    }

    if body.label.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "label is required");
    }
    if let Err(message) = validate_requested_permissions(&claims, &body.permissions) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) =
        validate_requested_clearance(&claims, body.classification_clearance.as_deref())
    {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_markings(
        &claims,
        &body.allowed_markings,
        body.classification_clearance.as_deref(),
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_org_scope(&claims, &body.allowed_org_ids) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_restricted_views(
        &state.db,
        &claims,
        &body.restricted_view_ids,
        ScopedSessionKind::Scoped,
    )
    .await
    {
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
        allowed_markings: body.allowed_markings,
        restricted_view_ids: body.restricted_view_ids,
        consumer_mode: body.consumer_mode,
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
    let requested_permissions = resolve_guest_permissions(&claims, &body.permissions);
    if let Err(message) = validate_requested_permissions(&claims, &requested_permissions) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if requested_permissions
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
    if let Err(message) = validate_requested_markings(
        &claims,
        &body.allowed_markings,
        body.classification_clearance.as_deref(),
    ) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_org_scope(&claims, &body.allowed_org_ids) {
        return json_error(StatusCode::FORBIDDEN, message);
    }
    if let Err(message) = validate_requested_restricted_views(
        &state.db,
        &claims,
        &body.restricted_view_ids,
        ScopedSessionKind::Guest,
    )
    .await
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
        allowed_markings: body.allowed_markings,
        restricted_view_ids: body.restricted_view_ids,
        consumer_mode: body.consumer_mode,
        guest_email: Some(body.guest_email.clone()),
        guest_display_name: body.guest_name.clone(),
    };

    match sessions::issue_scoped_session(
        &state.db,
        &state.jwt_config,
        &user,
        body.label.trim(),
        ScopedSessionKind::Guest,
        requested_permissions,
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
        return json_error(StatusCode::FORBIDDEN, "missing permission sessions:self");
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
    let requested_rank = access::marking_rank(requested_clearance)
        .ok_or_else(|| "unsupported classification_clearance".to_string())?;
    let caller_rank = claims
        .classification_clearance()
        .and_then(access::marking_rank)
        .unwrap_or_else(|| if claims.has_role("admin") { 2 } else { 0 });

    if claims.has_role("admin") || requested_rank <= caller_rank {
        Ok(())
    } else {
        Err("requested classification_clearance exceeds caller clearance".to_string())
    }
}

fn validate_requested_markings(
    claims: &auth_middleware::Claims,
    requested_markings: &[String],
    requested_clearance: Option<&str>,
) -> Result<(), String> {
    let normalized = access::normalize_markings(requested_markings)?;
    if normalized.is_empty() {
        return Ok(());
    }

    if let Some(clearance) = requested_clearance {
        let requested_rank = access::marking_rank(clearance)
            .ok_or_else(|| "unsupported classification_clearance".to_string())?;
        if normalized
            .iter()
            .any(|marking| access::marking_rank(marking).unwrap_or(0) > requested_rank)
        {
            return Err(
                "requested allowed_markings exceed requested classification_clearance".to_string(),
            );
        }
    }

    if claims.has_role("admin")
        || normalized
            .iter()
            .all(|marking| claims.allows_marking(marking))
    {
        Ok(())
    } else {
        Err("requested allowed_markings exceed caller clearance".to_string())
    }
}

fn validate_requested_org_scope(
    claims: &auth_middleware::Claims,
    requested_org_ids: &[Uuid],
) -> Result<(), String> {
    if claims.has_role("admin")
        || requested_org_ids.is_empty()
        || requested_org_ids
            .iter()
            .all(|org_id| claims.allows_org_id(Some(*org_id)))
    {
        Ok(())
    } else {
        Err("requested organization scope exceeds caller isolation boundary".to_string())
    }
}

fn permission_is_guest_safe(permission: &str) -> bool {
    permission.ends_with(":read") || permission.ends_with(":self") || permission == "*:read"
}

fn resolve_guest_permissions(
    claims: &auth_middleware::Claims,
    requested_permissions: &[String],
) -> Vec<String> {
    let mut permissions = if requested_permissions.is_empty() {
        claims
            .permissions
            .iter()
            .filter(|permission| permission_is_guest_safe(permission))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        requested_permissions.to_vec()
    };

    permissions.sort();
    permissions.dedup();
    permissions
}

async fn validate_requested_restricted_views(
    pool: &sqlx::PgPool,
    claims: &auth_middleware::Claims,
    restricted_view_ids: &[Uuid],
    session_kind: ScopedSessionKind,
) -> Result<(), String> {
    if restricted_view_ids.is_empty() {
        return Ok(());
    }

    let caller_restricted_views = claims.restricted_view_ids();
    if !claims.has_role("admin")
        && !caller_restricted_views.is_empty()
        && !restricted_view_ids
            .iter()
            .all(|view_id| caller_restricted_views.contains(view_id))
    {
        return Err("requested restricted views exceed caller session restrictions".to_string());
    }

    let rows = sqlx::query_as::<_, RestrictedViewRow>(
        r#"SELECT id, name, description, resource, action, conditions, row_filter, hidden_columns,
		          allowed_org_ids, allowed_markings, consumer_mode_enabled, allow_guest_access,
		          enabled, created_by, created_at, updated_at
		   FROM restricted_views
		   WHERE id = ANY($1) AND enabled = true"#,
    )
    .bind(restricted_view_ids.to_vec())
    .fetch_all(pool)
    .await
    .map_err(|error| format!("failed to validate restricted views: {error}"))?;

    if rows.len() != restricted_view_ids.len() {
        return Err("one or more restricted_view_ids do not exist or are disabled".to_string());
    }

    let views = rows
        .into_iter()
        .map(RestrictedView::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    for view in views {
        if session_kind == ScopedSessionKind::Guest && !view.allow_guest_access {
            return Err(format!(
                "restricted view '{}' does not allow guest access",
                view.name
            ));
        }
        if !claims.has_role("admin")
            && !view.allowed_org_ids.is_empty()
            && !view
                .allowed_org_ids
                .iter()
                .all(|org_id| claims.allows_org_id(Some(*org_id)))
        {
            return Err(format!(
                "restricted view '{}' exceeds caller organization boundary",
                view.name
            ));
        }
        if !claims.has_role("admin")
            && !view.allowed_markings.is_empty()
            && !view
                .allowed_markings
                .iter()
                .all(|marking| claims.allows_marking(marking))
        {
            return Err(format!(
                "restricted view '{}' exceeds caller classification boundary",
                view.name
            ));
        }
    }

    Ok(())
}

async fn load_current_user(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
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
            iss: None,
            aud: None,
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

    #[test]
    fn guest_permissions_default_to_safe_subset() {
        let claims = auth_middleware::Claims {
            permissions: vec![
                "datasets:read".to_string(),
                "datasets:write".to_string(),
                "reports:self".to_string(),
            ],
            ..claims_with_clearance("public")
        };

        assert_eq!(
            resolve_guest_permissions(&claims, &[]),
            vec!["datasets:read".to_string(), "reports:self".to_string()]
        );
    }
}
