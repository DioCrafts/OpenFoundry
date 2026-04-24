use crate::models::{agent::AgentMemorySnapshot, knowledge_base::KnowledgeSearchResult};

pub fn update_memory(
    current: &AgentMemorySnapshot,
    user_message: &str,
    final_response: &str,
    knowledge_hits: &[KnowledgeSearchResult],
) -> AgentMemorySnapshot {
    let mut short_term_notes = current.short_term_notes.clone();
    short_term_notes.push(truncate(user_message, 120));
    short_term_notes.truncate(6);

    let mut long_term_references = current.long_term_references.clone();
    for title in knowledge_hits.iter().map(|hit| hit.document_title.clone()) {
        if !long_term_references
            .iter()
            .any(|existing| existing == &title)
        {
            long_term_references.push(title);
        }
    }
    long_term_references.truncate(8);

    AgentMemorySnapshot {
        short_term_notes,
        long_term_references,
        last_run_summary: Some(truncate(final_response, 180)),
    }
}

fn truncate(content: &str, limit: usize) -> String {
    let mut chars = content.chars();
    let truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
