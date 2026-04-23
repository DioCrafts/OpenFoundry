use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub sql: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSavedQueryRequest {
    pub name: String,
    pub description: Option<String>,
    pub sql: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteQueryRequest {
    pub sql: String,
    pub limit: Option<usize>,
    pub execution_mode: Option<String>,
    pub distributed_worker_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ListQueriesQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}
