use std::cmp::Ordering;

use auth_middleware::claims::Claims;

use crate::{
    AppState,
    domain::indexer,
    models::search::{SearchRequest, SearchResult},
};

pub mod fulltext;
pub mod semantic;

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

    let mut results = documents
        .into_iter()
        .filter_map(|document| {
            let fulltext_score = fulltext::score(query, &document.title, &document.body);
            let semantic_score = if semantic_enabled {
                semantic::score(query, &format!("{} {}", document.title, document.body))
            } else {
                0.0
            };
            let title_bonus = if document
                .title
                .to_lowercase()
                .starts_with(&query.to_lowercase())
            {
                0.2
            } else {
                0.0
            };

            let score = (fulltext_score * 0.7) + (semantic_score.max(0.0) * 0.3) + title_bonus;
            if fulltext_score < 0.05 && semantic_score < 0.55 {
                return None;
            }

            Some(SearchResult {
                kind: document.kind,
                id: document.id,
                object_type_id: document.object_type_id,
                title: document.title,
                subtitle: document.subtitle,
                snippet: document.snippet,
                score,
                route: document.route,
                metadata: document.metadata,
            })
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    results.truncate(limit);

    Ok(results)
}
