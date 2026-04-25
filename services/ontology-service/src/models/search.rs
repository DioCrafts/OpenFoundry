use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub kind: Option<String>,
    pub object_type_id: Option<Uuid>,
    pub limit: Option<usize>,
    pub semantic: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub kind: String,
    pub id: Uuid,
    pub object_type_id: Option<Uuid>,
    pub title: String,
    pub subtitle: Option<String>,
    pub snippet: String,
    pub score: f32,
    pub route: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub data: Vec<SearchResult>,
}
