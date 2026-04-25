use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::decode_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusSpace {
    pub id: uuid::Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub space_kind: String,
    pub owner_peer_id: Option<uuid::Uuid>,
    pub region: String,
    pub member_peer_ids: Vec<uuid::Uuid>,
    pub governance_tags: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SpaceRow {
    pub id: uuid::Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub space_kind: String,
    pub owner_peer_id: Option<uuid::Uuid>,
    pub region: String,
    pub member_peer_ids: Value,
    pub governance_tags: Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<SpaceRow> for NexusSpace {
    type Error = String;

    fn try_from(row: SpaceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            slug: row.slug,
            display_name: row.display_name,
            description: row.description,
            space_kind: row.space_kind,
            owner_peer_id: row.owner_peer_id,
            region: row.region,
            member_peer_ids: decode_json(row.member_peer_ids, "member_peer_ids")?,
            governance_tags: decode_json(row.governance_tags, "governance_tags")?,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSpaceRequest {
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub space_kind: String,
    pub owner_peer_id: Option<uuid::Uuid>,
    pub region: String,
    #[serde(default)]
    pub member_peer_ids: Vec<uuid::Uuid>,
    #[serde(default)]
    pub governance_tags: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSpaceRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub owner_peer_id: Option<uuid::Uuid>,
    pub region: Option<String>,
    pub member_peer_ids: Option<Vec<uuid::Uuid>>,
    pub governance_tags: Option<Vec<String>>,
    pub status: Option<String>,
}
