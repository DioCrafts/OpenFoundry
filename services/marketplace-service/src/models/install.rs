use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::models::{decode_json, package::DependencyRequirement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallActivation {
    pub kind: String,
    pub status: String,
    pub resource_id: Option<Uuid>,
    pub resource_slug: Option<String>,
    pub public_url: Option<String>,
    pub notes: Option<String>,
}

impl Default for InstallActivation {
    fn default() -> Self {
        Self {
            kind: "marketplace_record".to_string(),
            status: "recorded".to_string(),
            resource_id: None,
            resource_slug: None,
            public_url: None,
            notes: Some("No runtime activation hook is configured for this package kind yet.".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRecord {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub version: String,
    pub workspace_name: String,
    pub status: String,
    pub dependency_plan: Vec<DependencyRequirement>,
    pub activation: InstallActivation,
    pub installed_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstallRequest {
    pub listing_id: Uuid,
    pub version: String,
    pub workspace_name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InstallRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub version: String,
    pub workspace_name: String,
    pub status: String,
    pub dependency_plan: Value,
    pub activation: Value,
    pub installed_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
}

impl TryFrom<InstallRow> for InstallRecord {
    type Error = String;

    fn try_from(row: InstallRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            listing_id: row.listing_id,
            listing_name: row.listing_name,
            version: row.version,
            workspace_name: row.workspace_name,
            status: row.status,
            dependency_plan: decode_json(row.dependency_plan, "dependency_plan")?,
            activation: if row.activation.is_null() || row.activation == json!({}) {
                InstallActivation::default()
            } else {
                decode_json(row.activation, "activation")?
            },
            installed_at: row.installed_at,
            ready_at: row.ready_at,
        })
    }
}
