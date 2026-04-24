use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryFile {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub path: String,
    pub branch_name: String,
    pub language: String,
    pub size_bytes: i32,
    pub content: String,
    pub last_commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub branch_name: String,
    pub snippet: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResponse {
    pub branch_name: String,
    pub patch: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileRow {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub path: String,
    pub branch_name: String,
    pub language: String,
    pub size_bytes: i32,
    pub content: String,
    pub last_commit_sha: String,
}

impl TryFrom<FileRow> for RepositoryFile {
    type Error = String;

    fn try_from(row: FileRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            repository_id: row.repository_id,
            path: row.path,
            branch_name: row.branch_name,
            language: row.language,
            size_bytes: row.size_bytes,
            content: row.content,
            last_commit_sha: row.last_commit_sha,
        })
    }
}
