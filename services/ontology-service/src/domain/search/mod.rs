use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use auth_middleware::claims::Claims;
use futures::{StreamExt, stream};

use crate::{
    AppState,
    domain::indexer::{self, SearchDocument},
    models::search::{SearchRequest, SearchResult, SearchScoreBreakdown},
};

pub mod fulltext;
pub mod semantic;

const DEFAULT_RRF_K: f32 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HybridStrategy {
    Rrf,
    Weighted,
}

#[derive(Debug, Clone)]
struct ScoredDocument {
    document: SearchDocument,
    lexical_score: f32,
    heuristic_semantic_score: f32,
    semantic_score: f32,
    title_bonus: f32,
}

pub async fn search_ontology(
    state: &AppState,
    claims: &Claims,
    request: &SearchRequest,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    let query = request.query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let documents = indexer::build_search_documents(
        state,
        claims,
        request.object_type_id,
        request.kind.as_deref(),
    )
    .await?;
    let semantic_enabled = request.semantic.unwrap_or(true);
    let limit = request.limit.unwrap_or(25).clamp(1, 100);
    let strategy = normalize_hybrid_strategy(request.hybrid_strategy.as_deref());

    let mut scored = documents
        .into_iter()
        .map(|document| {
            let semantic_text = semantic_text_for_document(&document);
            let fulltext_score = fulltext::score(query, &document.title, &document.body);
            let heuristic_semantic_score = semantic::score(query, &semantic_text);
            let title_bonus = title_prefix_bonus(query, &document.title);

            ScoredDocument {
                document,
                lexical_score: fulltext_score + title_bonus,
                heuristic_semantic_score,
                semantic_score: heuristic_semantic_score,
                title_bonus,
            }
        })
        .collect::<Vec<_>>();

    if scored.is_empty() {
        return Ok(Vec::new());
    }

    if semantic_enabled {
        let provider_backend =
            semantic::resolve_backend(state, request.embedding_provider.as_deref()).await?;
        let query_embedding = match semantic::embed_with_backend(state, &provider_backend, query)
            .await
        {
            Ok(embedding) if !embedding.is_empty() => Some(embedding),
            Ok(_) => None,
            Err(error) => {
                tracing::warn!(%error, provider = semantic::backend_reference(&provider_backend), "provider-backed ontology search embeddings failed, falling back to heuristic semantic ranking");
                None
            }
        };

        if let Some(query_embedding) = query_embedding {
            let pool = semantic_candidate_pool(&scored, limit, request.semantic_candidate_limit);
            let updated_scores = stream::iter(pool.iter().cloned())
                .map(|index| {
                    let semantic_text = semantic_text_for_document(&scored[index].document);
                    let query_embedding = query_embedding.clone();
                    let backend = provider_backend.clone();
                    async move {
                        let semantic_score = semantic::score_with_query_embedding(
                            state,
                            &backend,
                            &query_embedding,
                            &semantic_text,
                        )
                        .await;
                        (index, semantic_score)
                    }
                })
                .buffer_unordered(8)
                .collect::<Vec<_>>()
                .await;

            for (index, semantic_score) in updated_scores {
                match semantic_score {
                    Ok(score) => scored[index].semantic_score = score,
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            document_id = %scored[index].document.id,
                            provider = semantic::backend_reference(&provider_backend),
                            "semantic rerank embedding failed for ontology search document, keeping heuristic score"
                        );
                    }
                }
            }
        }
    }

    let results = match strategy {
        HybridStrategy::Rrf => fuse_with_rrf(&scored, limit),
        HybridStrategy::Weighted => fuse_with_weighted_scoring(&scored, limit),
    };

    Ok(results)
}

fn normalize_hybrid_strategy(value: Option<&str>) -> HybridStrategy {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some("weighted") => HybridStrategy::Weighted,
        _ => HybridStrategy::Rrf,
    }
}

fn title_prefix_bonus(query: &str, title: &str) -> f32 {
    if title.to_lowercase().starts_with(&query.to_lowercase()) {
        0.2
    } else {
        0.0
    }
}

fn semantic_text_for_document(document: &SearchDocument) -> String {
    let mut parts = Vec::new();
    if !document.title.trim().is_empty() {
        parts.push(document.title.trim().to_string());
    }
    if let Some(subtitle) = document
        .subtitle
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(subtitle.trim().to_string());
    }
    if !document.snippet.trim().is_empty() {
        parts.push(document.snippet.trim().to_string());
    }
    if !document.body.trim().is_empty() {
        parts.push(document.body.trim().to_string());
    }
    let combined = parts.join("\n");
    truncate_for_embeddings(&combined, 2_400)
}

fn truncate_for_embeddings(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    content.chars().take(max_chars).collect()
}

fn semantic_candidate_pool(
    scored: &[ScoredDocument],
    limit: usize,
    requested_pool_size: Option<usize>,
) -> Vec<usize> {
    let pool_size = requested_pool_size
        .unwrap_or(limit.saturating_mul(4).max(32))
        .clamp(limit.max(16), 160);

    let lexical_ranking = ranking_indices(scored, |left, right| {
        right
            .lexical_score
            .partial_cmp(&left.lexical_score)
            .unwrap_or(Ordering::Equal)
    });
    let heuristic_semantic_ranking = ranking_indices(scored, |left, right| {
        right
            .heuristic_semantic_score
            .partial_cmp(&left.heuristic_semantic_score)
            .unwrap_or(Ordering::Equal)
    });

    let mut selected = HashSet::new();
    for index in lexical_ranking.iter().take(pool_size) {
        selected.insert(*index);
    }
    for index in heuristic_semantic_ranking.iter().take(pool_size) {
        selected.insert(*index);
    }
    for (index, document) in scored.iter().enumerate() {
        if document.title_bonus > 0.0 {
            selected.insert(index);
        }
    }

    let mut pool = selected.into_iter().collect::<Vec<_>>();
    pool.sort_by(|left, right| {
        scored[*right]
            .lexical_score
            .partial_cmp(&scored[*left].lexical_score)
            .unwrap_or(Ordering::Equal)
    });
    pool
}

fn ranking_indices<F>(scored: &[ScoredDocument], mut compare: F) -> Vec<usize>
where
    F: FnMut(&ScoredDocument, &ScoredDocument) -> Ordering,
{
    let mut indices = (0..scored.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| compare(&scored[*left], &scored[*right]));
    indices
}

fn rank_map(indices: &[usize]) -> HashMap<usize, usize> {
    indices
        .iter()
        .enumerate()
        .map(|(position, index)| (*index, position + 1))
        .collect()
}

fn reciprocal_rank(rank: usize) -> f32 {
    1.0 / (DEFAULT_RRF_K + rank as f32)
}

fn passes_search_threshold(document: &ScoredDocument) -> bool {
    document.lexical_score >= 0.05 || document.semantic_score >= 0.55 || document.title_bonus > 0.0
}

fn fuse_with_rrf(scored: &[ScoredDocument], limit: usize) -> Vec<SearchResult> {
    let lexical_ranking = ranking_indices(scored, |left, right| {
        right
            .lexical_score
            .partial_cmp(&left.lexical_score)
            .unwrap_or(Ordering::Equal)
    });
    let semantic_ranking = ranking_indices(scored, |left, right| {
        right
            .semantic_score
            .partial_cmp(&left.semantic_score)
            .unwrap_or(Ordering::Equal)
    });
    let lexical_ranks = rank_map(&lexical_ranking);
    let semantic_ranks = rank_map(&semantic_ranking);

    let mut fused = scored
        .iter()
        .enumerate()
        .filter(|(_, document)| passes_search_threshold(document))
        .map(|(index, document)| {
            let lexical_rank = lexical_ranks.get(&index).copied();
            let semantic_rank = semantic_ranks.get(&index).copied();
            let lexical_rrf = lexical_rank.map(reciprocal_rank).unwrap_or_default();
            let semantic_rrf = semantic_rank.map(reciprocal_rank).unwrap_or_default();
            let score = (lexical_rrf * 1.1)
                + (semantic_rrf * 1.1)
                + (document.lexical_score * 0.25)
                + (document.semantic_score.max(0.0) * 0.25)
                + document.title_bonus;

            (document, lexical_rank, semantic_rank, score)
        })
        .collect::<Vec<_>>();

    fused.sort_by(|left, right| right.3.partial_cmp(&left.3).unwrap_or(Ordering::Equal));
    fused.truncate(limit);

    fused
        .into_iter()
        .map(
            |(document, lexical_rank, semantic_rank, score)| SearchResult {
                kind: document.document.kind.clone(),
                id: document.document.id,
                object_type_id: document.document.object_type_id,
                title: document.document.title.clone(),
                subtitle: document.document.subtitle.clone(),
                snippet: document.document.snippet.clone(),
                score,
                route: document.document.route.clone(),
                metadata: document.document.metadata.clone(),
                score_breakdown: Some(SearchScoreBreakdown {
                    fusion_strategy: "rrf".to_string(),
                    lexical_rank,
                    semantic_rank,
                    lexical_score: document.lexical_score,
                    semantic_score: document.semantic_score,
                    title_bonus: document.title_bonus,
                }),
            },
        )
        .collect()
}

fn fuse_with_weighted_scoring(scored: &[ScoredDocument], limit: usize) -> Vec<SearchResult> {
    let lexical_ranking = ranking_indices(scored, |left, right| {
        right
            .lexical_score
            .partial_cmp(&left.lexical_score)
            .unwrap_or(Ordering::Equal)
    });
    let semantic_ranking = ranking_indices(scored, |left, right| {
        right
            .semantic_score
            .partial_cmp(&left.semantic_score)
            .unwrap_or(Ordering::Equal)
    });
    let lexical_ranks = rank_map(&lexical_ranking);
    let semantic_ranks = rank_map(&semantic_ranking);

    let mut weighted = scored
        .iter()
        .enumerate()
        .filter(|(_, document)| passes_search_threshold(document))
        .map(|(index, document)| {
            let lexical_rank = lexical_ranks.get(&index).copied();
            let semantic_rank = semantic_ranks.get(&index).copied();
            let score = (document.lexical_score * 0.5)
                + (document.semantic_score.max(0.0) * 0.4)
                + (document.heuristic_semantic_score.max(0.0) * 0.1)
                + document.title_bonus;

            (document, lexical_rank, semantic_rank, score)
        })
        .collect::<Vec<_>>();

    weighted.sort_by(|left, right| right.3.partial_cmp(&left.3).unwrap_or(Ordering::Equal));
    weighted.truncate(limit);

    weighted
        .into_iter()
        .map(
            |(document, lexical_rank, semantic_rank, score)| SearchResult {
                kind: document.document.kind.clone(),
                id: document.document.id,
                object_type_id: document.document.object_type_id,
                title: document.document.title.clone(),
                subtitle: document.document.subtitle.clone(),
                snippet: document.document.snippet.clone(),
                score,
                route: document.document.route.clone(),
                metadata: document.document.metadata.clone(),
                score_breakdown: Some(SearchScoreBreakdown {
                    fusion_strategy: "weighted".to_string(),
                    lexical_rank,
                    semantic_rank,
                    lexical_score: document.lexical_score,
                    semantic_score: document.semantic_score,
                    title_bonus: document.title_bonus,
                }),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        ScoredDocument, fuse_with_rrf, semantic_candidate_pool, semantic_text_for_document,
    };
    use crate::domain::indexer::SearchDocument;

    fn sample_document(title: &str, body: &str) -> SearchDocument {
        SearchDocument {
            kind: "object_instance".to_string(),
            id: Uuid::nil(),
            object_type_id: None,
            title: title.to_string(),
            subtitle: None,
            snippet: body.to_string(),
            body: body.to_string(),
            route: "/ontology/test".to_string(),
            metadata: json!({}),
        }
    }

    #[test]
    fn semantic_text_includes_title_and_body() {
        let document = sample_document("Incident review", "Payment risk escalation");
        let text = semantic_text_for_document(&document);
        assert!(text.contains("Incident review"));
        assert!(text.contains("Payment risk escalation"));
    }

    #[test]
    fn candidate_pool_combines_lexical_and_semantic_recall() {
        let scored = vec![
            ScoredDocument {
                document: sample_document("A", "alpha"),
                lexical_score: 0.9,
                heuristic_semantic_score: 0.1,
                semantic_score: 0.1,
                title_bonus: 0.0,
            },
            ScoredDocument {
                document: sample_document("B", "beta"),
                lexical_score: 0.1,
                heuristic_semantic_score: 0.9,
                semantic_score: 0.9,
                title_bonus: 0.0,
            },
            ScoredDocument {
                document: sample_document("C", "gamma"),
                lexical_score: 0.2,
                heuristic_semantic_score: 0.2,
                semantic_score: 0.2,
                title_bonus: 0.0,
            },
        ];

        let pool = semantic_candidate_pool(&scored, 1, Some(1));
        assert!(pool.contains(&0));
        assert!(pool.contains(&1));
    }

    #[test]
    fn rrf_rewards_documents_present_in_both_rankings() {
        let scored = vec![
            ScoredDocument {
                document: sample_document("Strong lexical and semantic", "alpha"),
                lexical_score: 0.9,
                heuristic_semantic_score: 0.8,
                semantic_score: 0.8,
                title_bonus: 0.0,
            },
            ScoredDocument {
                document: sample_document("Lexical only", "beta"),
                lexical_score: 0.85,
                heuristic_semantic_score: 0.2,
                semantic_score: 0.2,
                title_bonus: 0.0,
            },
            ScoredDocument {
                document: sample_document("Semantic only", "gamma"),
                lexical_score: 0.2,
                heuristic_semantic_score: 0.85,
                semantic_score: 0.85,
                title_bonus: 0.0,
            },
        ];

        let results = fuse_with_rrf(&scored, 3);
        assert_eq!(results[0].title, "Strong lexical and semantic");
    }
}
