use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ObjectType {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub primary_key_property: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateObjectTypeRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub primary_key_property: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateObjectTypeRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub primary_key_property: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListObjectTypesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListObjectTypesResponse {
    pub data: Vec<ObjectType>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
