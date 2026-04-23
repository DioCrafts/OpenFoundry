use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Notebook {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: Uuid,
    pub default_kernel: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotebookRequest {
    pub name: String,
    pub description: Option<String>,
    pub default_kernel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotebookRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub default_kernel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotebooksQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}
