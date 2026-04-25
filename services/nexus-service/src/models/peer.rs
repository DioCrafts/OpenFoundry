use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::decode_json;

fn default_organization_type() -> String {
    "partner".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerOrganization {
    pub id: uuid::Uuid,
    pub slug: String,
    pub display_name: String,
    pub organization_type: String,
    pub region: String,
    pub endpoint_url: String,
    pub auth_mode: String,
    pub trust_level: String,
    pub public_key_fingerprint: String,
    pub shared_scopes: Vec<String>,
    pub status: String,
    pub lifecycle_stage: String,
    pub admin_contacts: Vec<String>,
    pub last_handshake_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PeerRow {
    pub id: uuid::Uuid,
    pub slug: String,
    pub display_name: String,
    pub organization_type: String,
    pub region: String,
    pub endpoint_url: String,
    pub auth_mode: String,
    pub trust_level: String,
    pub public_key_fingerprint: String,
    pub shared_scopes: Value,
    pub status: String,
    pub lifecycle_stage: String,
    pub admin_contacts: Value,
    pub last_handshake_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<PeerRow> for PeerOrganization {
    type Error = String;

    fn try_from(row: PeerRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            slug: row.slug,
            display_name: row.display_name,
            organization_type: row.organization_type,
            region: row.region,
            endpoint_url: row.endpoint_url,
            auth_mode: row.auth_mode,
            trust_level: row.trust_level,
            public_key_fingerprint: row.public_key_fingerprint,
            shared_scopes: decode_json(row.shared_scopes, "shared_scopes")?,
            status: row.status,
            lifecycle_stage: row.lifecycle_stage,
            admin_contacts: decode_json(row.admin_contacts, "admin_contacts")?,
            last_handshake_at: row.last_handshake_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePeerRequest {
    pub slug: String,
    pub display_name: String,
    #[serde(default = "default_organization_type")]
    pub organization_type: String,
    pub region: String,
    pub endpoint_url: String,
    pub auth_mode: String,
    pub trust_level: String,
    pub public_key_fingerprint: String,
    #[serde(default)]
    pub shared_scopes: Vec<String>,
    #[serde(default)]
    pub admin_contacts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePeerRequest {
    pub display_name: Option<String>,
    pub organization_type: Option<String>,
    pub region: Option<String>,
    pub endpoint_url: Option<String>,
    pub trust_level: Option<String>,
    pub shared_scopes: Option<Vec<String>>,
    pub status: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub admin_contacts: Option<Vec<String>>,
}
