use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Property {
    pub id: Uuid,
    pub object_type_id: Uuid,
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

#[derive(Debug, Deserialize)]
pub struct CreatePropertyRequest {
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
pub struct UpdatePropertyRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub unique_constraint: Option<bool>,
    pub time_dependent: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
}
