use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Cell {
    pub id: Uuid,
    pub notebook_id: Uuid,
    pub cell_type: String, // "code", "markdown"
    pub kernel: String,    // "python", "sql"
    pub source: String,
    pub position: i32,
    pub last_output: Option<serde_json::Value>,
    pub execution_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCellRequest {
    pub cell_type: Option<String>,
    pub kernel: Option<String>,
    pub source: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCellRequest {
    pub source: Option<String>,
    pub cell_type: Option<String>,
    pub kernel: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteCellRequest {
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CellOutput {
    pub output_type: String, // "text", "error", "table", "image"
    pub content: serde_json::Value,
    pub execution_count: i32,
}
