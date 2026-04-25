use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct RestrictedViewRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
    pub conditions: Value,
    pub row_filter: Option<String>,
    pub hidden_columns: Value,
    pub allowed_org_ids: Value,
    pub allowed_markings: Value,
    pub consumer_mode_enabled: bool,
    pub allow_guest_access: bool,
    pub enabled: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestrictedView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
    pub conditions: Value,
    pub row_filter: Option<String>,
    pub hidden_columns: Vec<String>,
    pub allowed_org_ids: Vec<Uuid>,
    pub allowed_markings: Vec<String>,
    pub consumer_mode_enabled: bool,
    pub allow_guest_access: bool,
    pub enabled: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<RestrictedViewRow> for RestrictedView {
    type Error = String;

    fn try_from(row: RestrictedViewRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            resource: row.resource,
            action: row.action,
            conditions: row.conditions,
            row_filter: row.row_filter,
            hidden_columns: serde_json::from_value(row.hidden_columns)
                .map_err(|error| format!("invalid hidden_columns: {error}"))?,
            allowed_org_ids: serde_json::from_value(row.allowed_org_ids)
                .map_err(|error| format!("invalid allowed_org_ids: {error}"))?,
            allowed_markings: serde_json::from_value(row.allowed_markings)
                .map_err(|error| format!("invalid allowed_markings: {error}"))?,
            consumer_mode_enabled: row.consumer_mode_enabled,
            allow_guest_access: row.allow_guest_access,
            enabled: row.enabled,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertRestrictedViewRequest {
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
    #[serde(default)]
    pub conditions: Value,
    pub row_filter: Option<String>,
    #[serde(default)]
    pub hidden_columns: Vec<String>,
    #[serde(default)]
    pub allowed_org_ids: Vec<Uuid>,
    #[serde(default)]
    pub allowed_markings: Vec<String>,
    #[serde(default)]
    pub consumer_mode_enabled: bool,
    #[serde(default)]
    pub allow_guest_access: bool,
    pub enabled: bool,
}
