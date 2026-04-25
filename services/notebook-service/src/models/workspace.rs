use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookWorkspaceFile {
    pub path: String,
    pub language: String,
    pub content: String,
    pub size_bytes: usize,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertNotebookWorkspaceFileRequest {
    pub path: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteNotebookWorkspaceFileQuery {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ListNotebookWorkspaceFilesResponse {
    pub data: Vec<NotebookWorkspaceFile>,
}
