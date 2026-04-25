use std::cmp::Ordering;

use crate::{
    domain::llm::cache,
    models::knowledge_base::{KnowledgeDocument, KnowledgeSearchResult},
};

pub fn search(
    query: &str,
    documents: &[KnowledgeDocument],
    top_k: usize,
    min_score: f32,
) -> Vec<KnowledgeSearchResult> {
    let query_embedding = crate::domain::rag::embedder::embed_text(query);
    search_with_embedding(&query_embedding, documents, top_k, min_score)
}

pub fn search_with_embedding(
    query_embedding: &[f32],
    documents: &[KnowledgeDocument],
    top_k: usize,
    min_score: f32,
) -> Vec<KnowledgeSearchResult> {
    let mut hits = Vec::new();

    for document in documents {
        for chunk in &document.chunks {
            let score = cache::cosine_similarity(&query_embedding, &chunk.embedding);
            if score < min_score {
                continue;
            }

            hits.push(KnowledgeSearchResult {
                knowledge_base_id: document.knowledge_base_id,
                document_id: document.id,
                document_title: document.title.clone(),
                chunk_id: chunk.id.clone(),
                score,
                excerpt: chunk.text.clone(),
                source_uri: document.source_uri.clone(),
                metadata: chunk.metadata.clone(),
            });
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    hits.truncate(top_k.max(1));
    hits
}
