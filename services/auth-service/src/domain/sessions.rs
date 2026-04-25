use auth_middleware::{
    claims::SessionScope,
    jwt::{self, JwtConfig},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{access, rbac},
    models::{
        session::{ScopedSession, ScopedSessionKind, ScopedSessionRow, ScopedSessionWithToken},
        user::User,
    },
};

#[derive(Debug)]
pub enum ScopedSessionError {
    Database(sqlx::Error),
    Token(auth_middleware::JwtError),
    InvalidExpiration,
}

impl From<sqlx::Error> for ScopedSessionError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<auth_middleware::JwtError> for ScopedSessionError {
    fn from(value: auth_middleware::JwtError) -> Self {
        Self::Token(value)
    }
}

pub async fn list_scoped_sessions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ScopedSession>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ScopedSessionRow>(
        r#"SELECT id, user_id, label, session_kind, scope, guest_email, guest_name, expires_at, revoked_at, created_at
           FROM scoped_sessions
           WHERE user_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(ScopedSession::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn issue_scoped_session(
    pool: &PgPool,
    config: &JwtConfig,
    user: &User,
    label: &str,
    session_kind: ScopedSessionKind,
    requested_permissions: Vec<String>,
    mut scope: SessionScope,
    expires_at: Option<DateTime<Utc>>,
    guest_email: Option<String>,
    guest_name: Option<String>,
) -> Result<ScopedSessionWithToken, ScopedSessionError> {
    let now = Utc::now();
    let expires_at = expires_at.unwrap_or_else(|| {
        now + if session_kind == ScopedSessionKind::Guest {
            chrono::Duration::hours(12)
        } else {
            chrono::Duration::hours(24)
        }
    });
    if expires_at <= now {
        return Err(ScopedSessionError::InvalidExpiration);
    }

    normalize_scope(&mut scope, user.organization_id, session_kind);

    let access_bundle = rbac::get_user_access_bundle(pool, user.id)
        .await
        .unwrap_or_default();
    let granted_permissions = if requested_permissions.is_empty() {
        access_bundle.permissions.clone()
    } else {
        requested_permissions
    };

    let session_id = Uuid::now_v7();
    let expires_in_secs = (expires_at - now).num_seconds().max(60);
    let token = jwt::encode_token(
        config,
        &jwt::build_access_claims_with_scope(
            &config.clone().with_access_ttl(expires_in_secs),
            user.id,
            &user.email,
            &user.name,
            if session_kind == ScopedSessionKind::Guest {
                vec!["guest".to_string()]
            } else {
                access_bundle.roles.clone()
            },
            granted_permissions,
            user.organization_id,
            user.attributes.clone(),
            vec![match session_kind {
                ScopedSessionKind::Scoped => "scoped_session".to_string(),
                ScopedSessionKind::Guest => "guest".to_string(),
            }],
            Some(scope.clone()),
            Some(match session_kind {
                ScopedSessionKind::Scoped => "scoped_session".to_string(),
                ScopedSessionKind::Guest => "guest_session".to_string(),
            }),
        ),
    )?;

    sqlx::query(
        r#"INSERT INTO scoped_sessions (id, user_id, label, session_kind, scope, guest_email, guest_name, expires_at)
           VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8)"#,
    )
    .bind(session_id)
    .bind(user.id)
    .bind(label)
    .bind(session_kind.as_str())
    .bind(serde_json::to_value(&scope).unwrap_or_default())
    .bind(guest_email.clone())
    .bind(guest_name.clone())
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(ScopedSessionWithToken {
        id: session_id,
        label: label.to_string(),
        session_kind,
        scope,
        token,
        expires_at,
        guest_email,
        guest_name,
        created_at: now,
    })
}

pub async fn revoke_scoped_session(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Uuid,
    allow_any_user: bool,
) -> Result<bool, sqlx::Error> {
    let result = if allow_any_user {
        sqlx::query(
            "UPDATE scoped_sessions SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE scoped_sessions SET revoked_at = NOW() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await?
    };

    Ok(result.rows_affected() > 0)
}

fn normalize_scope(
    scope: &mut SessionScope,
    user_org_id: Option<Uuid>,
    session_kind: ScopedSessionKind,
) {
    scope.allowed_methods = normalize_methods(&scope.allowed_methods);
    scope.allowed_path_prefixes = normalize_paths(&scope.allowed_path_prefixes);
    scope.allowed_subject_ids.sort();
    scope.allowed_subject_ids.dedup();
    scope.allowed_org_ids.sort();
    scope.allowed_org_ids.dedup();
    scope.allowed_markings =
        access::normalize_markings(&scope.allowed_markings).unwrap_or_default();
    scope.restricted_view_ids.sort();
    scope.restricted_view_ids.dedup();

    if scope.allowed_org_ids.is_empty() {
        if let Some(org_id) = user_org_id {
            scope.allowed_org_ids.push(org_id);
        }
    }

    if scope.classification_clearance.is_none() {
        scope.classification_clearance = access::max_marking(&scope.allowed_markings);
    }
    if scope.allowed_markings.is_empty() {
        scope.allowed_markings =
            access::markings_for_clearance(scope.classification_clearance.as_deref());
    }

    if session_kind == ScopedSessionKind::Guest {
        if scope.allowed_methods.is_empty() {
            scope.allowed_methods = vec!["GET".to_string()];
        }
        if scope.allowed_path_prefixes.is_empty() {
            scope.allowed_path_prefixes = vec![
                "/api/v1/datasets".to_string(),
                "/api/v1/lineage".to_string(),
                "/api/v1/ontology".to_string(),
                "/api/v1/reports".to_string(),
                "/api/v1/audit".to_string(),
            ];
        }
        if scope.classification_clearance.is_none() {
            scope.classification_clearance = Some("public".to_string());
        }
        if scope.allowed_markings.is_empty() {
            scope.allowed_markings = vec!["public".to_string()];
        }
        scope.consumer_mode = true;
    }
}

fn normalize_methods(values: &[String]) -> Vec<String> {
    let mut methods = values
        .iter()
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    methods.sort();
    methods.dedup();
    methods
}

fn normalize_paths(values: &[String]) -> Vec<String> {
    let mut paths = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.starts_with('/') {
                value.to_string()
            } else {
                format!("/{value}")
            }
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_guest_scope_sets_safe_defaults() {
        let mut scope = SessionScope::default();
        normalize_scope(&mut scope, Some(Uuid::nil()), ScopedSessionKind::Guest);
        assert_eq!(scope.allowed_methods, vec!["GET".to_string()]);
        assert!(
            scope
                .allowed_path_prefixes
                .iter()
                .any(|prefix| prefix == "/api/v1/datasets")
        );
        assert_eq!(scope.allowed_org_ids, vec![Uuid::nil()]);
        assert_eq!(scope.classification_clearance.as_deref(), Some("public"));
        assert_eq!(scope.allowed_markings, vec!["public".to_string()]);
        assert!(scope.consumer_mode);
    }

    #[test]
    fn normalize_paths_and_methods_deduplicate() {
        assert_eq!(
            normalize_methods(&["get".into(), "GET".into(), "post".into()]),
            vec!["GET".to_string(), "POST".to_string()]
        );
        assert_eq!(
            normalize_paths(&["api/v1/datasets".into(), "/api/v1/datasets".into()]),
            vec!["/api/v1/datasets".to_string()]
        );
    }
}
