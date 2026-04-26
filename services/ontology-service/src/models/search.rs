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
    pub hybrid_strategy: Option<String>,
    pub embedding_provider: Option<String>,
    pub semantic_candidate_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchScoreBreakdown {
    pub fusion_strategy: String,
    pub lexical_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    pub lexical_score: f32,
    pub semantic_score: f32,
    pub title_bonus: f32,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<SearchScoreBreakdown>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub data: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnnObjectsRequest {
    pub property_name: String,
    pub anchor_object_id: Option<Uuid>,
    pub query_vector: Option<Vec<f32>>,
    pub limit: Option<usize>,
    pub metric: Option<String>,
    pub exclude_anchor: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnnObjectResult {
    pub object: Value,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnnObjectsResponse {
    pub property_name: String,
    pub metric: String,
    pub total: usize,
    pub data: Vec<KnnObjectResult>,
}
