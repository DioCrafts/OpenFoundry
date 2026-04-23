use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    Timestamp,
    Json,
    Array,
    Reference,
}

impl std::fmt::Display for PropertyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Timestamp => "timestamp",
            Self::Json => "json",
            Self::Array => "array",
            Self::Reference => "reference",
        };
        write!(f, "{s}")
    }
}

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
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePropertyRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub unique_constraint: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
}
