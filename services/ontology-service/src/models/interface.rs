use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OntologyInterface {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InterfaceProperty {
    pub id: Uuid,
    pub interface_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub property_type: String,
    pub required: bool,
    pub unique_constraint: bool,
    pub time_dependent: bool,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ObjectTypeInterfaceBinding {
    pub object_type_id: Uuid,
    pub interface_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInterfaceRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInterfaceRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListInterfacesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListInterfacesResponse {
    pub data: Vec<OntologyInterface>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateInterfacePropertyRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub property_type: String,
    pub required: Option<bool>,
    pub unique_constraint: Option<bool>,
    pub time_dependent: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInterfacePropertyRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub unique_constraint: Option<bool>,
    pub time_dependent: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
}
