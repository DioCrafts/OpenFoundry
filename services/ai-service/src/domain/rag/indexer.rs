use serde_json::json;
use uuid::Uuid;

use crate::models::knowledge_base::KnowledgeChunk;

use super::{chunker, embedder};

pub fn index_document(
    document_id: Uuid,
    content: &str,
    chunking_strategy: &str,
) -> Vec<KnowledgeChunk> {
    let max_chars = if chunking_strategy == "fine" {
        320
    } else {
        520
    };

    chunker::chunk_text(content, max_chars)
        .into_iter()
        .map(|(position, text)| KnowledgeChunk {
            id: format!("{}-{position}", document_id),
            position,
            text: text.clone(),
            token_count: text.split_whitespace().count() as i32,
            embedding: embedder::embed_text(&text),
            metadata: json!({ "strategy": chunking_strategy }),
        })
        .collect()
}
