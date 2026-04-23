use serde_json::Value;
use uuid::Uuid;

use crate::claims::Claims;

/// Row-level security context derived from user claims.
/// Services use this to scope DB queries to the user's org/permissions.
#[derive(Debug, Clone)]
pub struct RlsContext {
    pub user_id: Uuid,
    pub org_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub attributes: Value,
}

impl From<&Claims> for RlsContext {
    fn from(claims: &Claims) -> Self {
        Self {
            user_id: claims.sub,
            org_id: claims.org_id,
            roles: claims.roles.clone(),
            permissions: claims.permissions.clone(),
            attributes: claims.attributes.clone(),
        }
    }
}

impl RlsContext {
    /// Returns true if the user is an admin (bypasses row-level checks).
    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r == "admin")
    }

    /// Returns true if a permission key is present.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.is_admin() || self.permissions.iter().any(|candidate| candidate == permission)
    }

    /// SQL fragment for filtering by org_id. Returns "TRUE" for admins.
    pub fn org_filter(&self, column: &str) -> String {
        if self.is_admin() {
            "TRUE".to_string()
        } else if let Some(org) = self.org_id {
            format!("{column} = '{org}'")
        } else {
            format!("{column} IS NULL")
        }
    }

    /// SQL fragment that scopes access to either owner or organization.
    pub fn owner_or_org_filter(&self, owner_column: &str, org_column: &str) -> String {
        if self.is_admin() || self.has_permission("rows:all") {
            "TRUE".to_string()
        } else if let Some(org) = self.org_id {
            format!("({owner_column} = '{}' OR {org_column} = '{org}')", self.user_id)
        } else {
            format!("{owner_column} = '{}'", self.user_id)
        }
    }
}
