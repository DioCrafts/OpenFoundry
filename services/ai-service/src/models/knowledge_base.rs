use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    pub id: String,
    pub position: i32,
    pub text: String,
    pub token_count: i32,
    #[serde(default)]
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub title: String,
    pub content: String,
    pub source_uri: Option<String>,
    pub metadata: Value,
    pub status: String,
    pub chunk_count: i32,
    pub chunks: Vec<KnowledgeChunk>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub embedding_provider: String,
    pub chunking_strategy: String,
    pub tags: Vec<String>,
    pub document_count: i64,
    pub chunk_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchResult {
    pub knowledge_base_id: Uuid,
    pub document_id: Uuid,
    pub document_title: String,
    pub chunk_id: String,
    pub score: f32,
    pub excerpt: String,
    pub source_uri: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListKnowledgeBasesResponse {
    pub data: Vec<KnowledgeBase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListKnowledgeDocumentsResponse {
    pub data: Vec<KnowledgeDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_knowledge_status")]
    pub status: String,
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    #[serde(default = "default_chunking_strategy")]
    pub chunking_strategy: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateKnowledgeBaseRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub embedding_provider: Option<String>,
    pub chunking_strategy: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgeDocumentRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub source_uri: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKnowledgeBaseRequest {
    pub query: String,
    #[serde(default = "default_search_top_k")]
    pub top_k: usize,
    #[serde(default = "default_min_score")]
    pub min_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKnowledgeBaseResponse {
    pub knowledge_base_id: Uuid,
    pub query: String,
    pub results: Vec<KnowledgeSearchResult>,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct KnowledgeBaseRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub embedding_provider: String,
    pub chunking_strategy: String,
    pub tags: Json<Vec<String>>,
    pub document_count: i64,
    pub chunk_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct KnowledgeDocumentRow {
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub title: String,
    pub content: String,
    pub source_uri: Option<String>,
    pub metadata: Json<Value>,
    pub status: String,
    pub chunk_count: i32,
    pub chunks: Json<Vec<KnowledgeChunk>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<KnowledgeBaseRow> for KnowledgeBase {
    fn from(value: KnowledgeBaseRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            status: value.status,
            embedding_provider: value.embedding_provider,
            chunking_strategy: value.chunking_strategy,
            tags: value.tags.0,
            document_count: value.document_count,
            chunk_count: value.chunk_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<KnowledgeDocumentRow> for KnowledgeDocument {
    fn from(value: KnowledgeDocumentRow) -> Self {
        Self {
            id: value.id,
            knowledge_base_id: value.knowledge_base_id,
            title: value.title,
            content: value.content,
            source_uri: value.source_uri,
            metadata: value.metadata.0,
            status: value.status,
            chunk_count: value.chunk_count,
            chunks: value.chunks.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn default_knowledge_status() -> String {
    "active".to_string()
}

fn default_embedding_provider() -> String {
    "deterministic-hash".to_string()
}

fn default_chunking_strategy() -> String {
    "balanced".to_string()
}

fn default_search_top_k() -> usize {
    5
}

fn default_min_score() -> f32 {
    0.55
}
