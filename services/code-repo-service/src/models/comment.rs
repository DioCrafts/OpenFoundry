use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: Uuid,
    pub merge_request_id: Uuid,
    pub author: String,
    pub body: String,
    pub file_path: String,
    pub line_number: Option<i32>,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub author: String,
    pub body: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub line_number: Option<i32>,
    #[serde(default)]
    pub resolved: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentRow {
    pub id: Uuid,
    pub merge_request_id: Uuid,
    pub author: String,
    pub body: String,
    pub file_path: String,
    pub line_number: Option<i32>,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<CommentRow> for ReviewComment {
    type Error = String;

    fn try_from(row: CommentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            merge_request_id: row.merge_request_id,
            author: row.author,
            body: row.body,
            file_path: row.file_path,
            line_number: row.line_number,
            resolved: row.resolved,
            created_at: row.created_at,
        })
    }
}
