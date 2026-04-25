use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SessionScope {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_methods: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_path_prefixes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_subject_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_org_ids: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_clearance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_display_name: Option<String>,
}

/// JWT claims embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user ID (UUID v7).
    pub sub: Uuid,
    /// Issued at (epoch seconds).
    pub iat: i64,
    /// Expiration (epoch seconds).
    pub exp: i64,
    /// Token ID — unique per token for revocation.
    pub jti: Uuid,
    /// User email.
    pub email: String,
    /// Display name.
    pub name: String,
    /// Roles assigned to the user.
    pub roles: Vec<String>,
    /// Effective permissions assigned to the subject.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Optional organization/tenant scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<Uuid>,
    /// Arbitrary subject attributes used by ABAC policies.
    #[serde(default)]
    pub attributes: Value,
    /// Authentication methods used for this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auth_methods: Vec<String>,
    /// Token intent, e.g. access, refresh, or api_key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_use: Option<String>,
    /// API key identifier when the claims were issued for programmatic access.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<Uuid>,
    /// Optional issued session kind such as `scoped_session` or `guest_session`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_kind: Option<String>,
    /// Optional zero-trust restrictions and guest/session scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_scope: Option<SessionScope>,
}

impl Claims {
    /// Check if the token has expired.
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }

    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user has any of the given roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }

    /// Check if the subject has a permission key, e.g. `users:read`.
    pub fn has_permission_key(&self, permission: &str) -> bool {
        let resource_wildcard = permission
            .split_once(':')
            .map(|(resource, _)| format!("{resource}:*"));

        self.has_role("admin")
            || self.permissions.iter().any(|candidate| {
                candidate == permission
                    || candidate == "*:*"
                    || resource_wildcard
                        .as_ref()
                        .is_some_and(|wildcard| candidate == wildcard)
            })
    }

    /// Check if the subject can perform an action over a resource.
    pub fn has_permission(&self, resource: &str, action: &str) -> bool {
        self.has_permission_key(&format!("{resource}:{action}"))
    }

    /// Whether the session was issued as an external/guest session.
    pub fn is_guest_session(&self) -> bool {
        self.session_kind.as_deref() == Some("guest_session")
            || self
                .session_scope
                .as_ref()
                .and_then(|scope| scope.guest_email.as_deref())
                .is_some()
    }

    /// Effective classification clearance, preferring explicit session scope over attributes.
    pub fn classification_clearance(&self) -> Option<&str> {
        self.session_scope
            .as_ref()
            .and_then(|scope| scope.classification_clearance.as_deref())
            .or_else(|| {
                self.attribute("classification_clearance")
                    .and_then(Value::as_str)
            })
    }

    /// Whether the session scope permits an HTTP method.
    pub fn allows_http_method(&self, method: &str) -> bool {
        let Some(scope) = self.session_scope.as_ref() else {
            return true;
        };
        if scope.allowed_methods.is_empty() {
            return true;
        }
        scope.allowed_methods.iter().any(|candidate| {
            candidate.eq_ignore_ascii_case(method)
                || candidate.eq_ignore_ascii_case("*")
        })
    }

    /// Whether the session scope permits the requested path prefix.
    pub fn allows_path(&self, path: &str) -> bool {
        let Some(scope) = self.session_scope.as_ref() else {
            return true;
        };
        if scope.allowed_path_prefixes.is_empty() {
            return true;
        }
        scope
            .allowed_path_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    /// Whether the session scope permits the given subject identifier.
    pub fn allows_subject_id(&self, subject_id: Option<&str>) -> bool {
        let Some(scope) = self.session_scope.as_ref() else {
            return true;
        };
        if scope.allowed_subject_ids.is_empty() {
            return true;
        }
        subject_id.is_some_and(|candidate| {
            scope.allowed_subject_ids.iter().any(|value| value == candidate)
        })
    }

    /// Effective organization allowlist for restricted sessions.
    pub fn allowed_org_ids(&self) -> Vec<Uuid> {
        if let Some(scope) = &self.session_scope {
            if !scope.allowed_org_ids.is_empty() {
                return scope.allowed_org_ids.clone();
            }
        }
        self.org_id.into_iter().collect()
    }

    /// Fetch an attribute if present.
    pub fn attribute(&self, key: &str) -> Option<&Value> {
        self.attributes.as_object().and_then(|map| map.get(key))
    }

    /// Get the issued-at time as a DateTime.
    pub fn issued_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.iat, 0)
    }

    /// Get the expiration time as a DateTime.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.exp, 0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn scoped_claims() -> Claims {
        Claims {
            sub: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            jti: Uuid::nil(),
            email: "guest@example.com".to_string(),
            name: "Guest".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: vec!["datasets:read".to_string()],
            org_id: Some(Uuid::nil()),
            attributes: json!({
                "classification_clearance": "confidential"
            }),
            auth_methods: vec!["guest".to_string()],
            token_use: Some("access".to_string()),
            api_key_id: None,
            session_kind: Some("guest_session".to_string()),
            session_scope: Some(SessionScope {
                allowed_methods: vec!["GET".to_string()],
                allowed_path_prefixes: vec!["/api/v1/datasets".to_string()],
                allowed_subject_ids: vec!["subject-1".to_string()],
                allowed_org_ids: vec![Uuid::nil()],
                workspace: Some("shared".to_string()),
                classification_clearance: Some("public".to_string()),
                guest_email: Some("guest@example.com".to_string()),
                guest_display_name: Some("Guest".to_string()),
            }),
        }
    }

    #[test]
    fn session_scope_limits_methods_and_paths() {
        let claims = scoped_claims();
        assert!(claims.is_guest_session());
        assert!(claims.allows_http_method("GET"));
        assert!(!claims.allows_http_method("POST"));
        assert!(claims.allows_path("/api/v1/datasets/123"));
        assert!(!claims.allows_path("/api/v1/pipelines"));
    }

    #[test]
    fn session_scope_limits_subjects_and_prefers_scope_clearance() {
        let claims = scoped_claims();
        assert_eq!(claims.classification_clearance(), Some("public"));
        assert!(claims.allows_subject_id(Some("subject-1")));
        assert!(!claims.allows_subject_id(Some("subject-2")));
        assert_eq!(claims.allowed_org_ids(), vec![Uuid::nil()]);
    }
}
