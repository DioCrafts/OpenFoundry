use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LinkType {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub source_type_id: Uuid,
    pub target_type_id: Uuid,
    pub cardinality: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkTypeRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub source_type_id: Uuid,
    pub target_type_id: Uuid,
    pub cardinality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLinkTypeRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub cardinality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListLinkTypesQuery {
    pub object_type_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
