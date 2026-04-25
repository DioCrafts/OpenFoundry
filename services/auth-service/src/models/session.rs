use auth_middleware::claims::SessionScope;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopedSessionKind {
    Scoped,
    Guest,
}

impl ScopedSessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scoped => "scoped",
            Self::Guest => "guest",
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ScopedSessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub session_kind: String,
    pub scope: serde_json::Value,
    pub guest_email: Option<String>,
    pub guest_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    pub session_kind: ScopedSessionKind,
    pub scope: SessionScope,
    pub guest_email: Option<String>,
    pub guest_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedSessionWithToken {
    pub id: Uuid,
    pub label: String,
    pub session_kind: ScopedSessionKind,
    pub scope: SessionScope,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub guest_email: Option<String>,
    pub guest_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateScopedSessionRequest {
    pub label: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    #[serde(default)]
    pub allowed_path_prefixes: Vec<String>,
    #[serde(default)]
    pub allowed_subject_ids: Vec<String>,
    #[serde(default)]
    pub allowed_org_ids: Vec<Uuid>,
    pub workspace: Option<String>,
    pub classification_clearance: Option<String>,
    #[serde(default)]
    pub allowed_markings: Vec<String>,
    #[serde(default)]
    pub restricted_view_ids: Vec<Uuid>,
    #[serde(default)]
    pub consumer_mode: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGuestSessionRequest {
    pub label: String,
    pub guest_email: String,
    pub guest_name: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    #[serde(default)]
    pub allowed_path_prefixes: Vec<String>,
    #[serde(default)]
    pub allowed_subject_ids: Vec<String>,
    #[serde(default)]
    pub allowed_org_ids: Vec<Uuid>,
    pub workspace: Option<String>,
    pub classification_clearance: Option<String>,
    #[serde(default)]
    pub allowed_markings: Vec<String>,
    #[serde(default)]
    pub restricted_view_ids: Vec<Uuid>,
    #[serde(default)]
    pub consumer_mode: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl TryFrom<ScopedSessionRow> for ScopedSession {
    type Error = String;

    fn try_from(row: ScopedSessionRow) -> Result<Self, Self::Error> {
        let session_kind = match row.session_kind.as_str() {
            "scoped" => ScopedSessionKind::Scoped,
            "guest" => ScopedSessionKind::Guest,
            other => return Err(format!("unsupported scoped session kind: {other}")),
        };
        let scope =
            serde_json::from_value::<SessionScope>(row.scope).map_err(|error| error.to_string())?;

        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            label: row.label,
            session_kind,
            scope,
            guest_email: row.guest_email,
            guest_name: row.guest_name,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
        })
    }
}
