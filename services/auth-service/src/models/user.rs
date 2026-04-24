use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub is_active: bool,
    pub organization_id: Option<Uuid>,
    pub attributes: Value,
    pub mfa_enforced: bool,
    pub auth_source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public-facing user representation (no password hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub permissions: Vec<String>,
    pub organization_id: Option<Uuid>,
    pub attributes: Value,
    pub mfa_enabled: bool,
    pub mfa_enforced: bool,
    pub auth_source: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn into_response(self, roles: Vec<String>) -> UserResponse {
        UserResponse {
            id: self.id,
            email: self.email,
            name: self.name,
            is_active: self.is_active,
            roles,
            groups: vec![],
            permissions: vec![],
            organization_id: self.organization_id,
            attributes: self.attributes,
            mfa_enabled: false,
            mfa_enforced: self.mfa_enforced,
            auth_source: self.auth_source,
            created_at: self.created_at,
        }
    }
}
