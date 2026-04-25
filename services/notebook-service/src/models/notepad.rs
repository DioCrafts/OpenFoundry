use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotepadDocument {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub owner_id: Uuid,
    pub content: String,
    pub template_key: Option<String>,
    pub widgets: Value,
    pub last_indexed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotepadPresence {
    pub id: Uuid,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub display_name: String,
    pub cursor_label: String,
    pub color: String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotepadDocumentsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotepadDocumentRequest {
    pub title: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub template_key: Option<String>,
    pub widgets: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotepadDocumentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub template_key: Option<String>,
    pub widgets: Option<Value>,
    pub last_indexed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertNotepadPresenceRequest {
    pub session_id: String,
    pub display_name: String,
    pub cursor_label: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotepadExportPayload {
    pub file_name: String,
    pub mime_type: String,
    pub title: String,
    pub html: String,
    pub preview_excerpt: String,
}
