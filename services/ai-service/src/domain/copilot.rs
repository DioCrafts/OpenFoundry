use uuid::Uuid;

use crate::models::knowledge_base::KnowledgeSearchResult;

#[derive(Debug, Clone)]
pub struct CopilotDraft {
    pub answer: String,
    pub suggested_sql: Option<String>,
    pub pipeline_suggestions: Vec<String>,
    pub ontology_hints: Vec<String>,
}

pub fn assist(
    question: &str,
    dataset_ids: &[Uuid],
    ontology_type_ids: &[Uuid],
    knowledge_hits: &[KnowledgeSearchResult],
    include_sql: bool,
    include_pipeline_plan: bool,
) -> CopilotDraft {
    let lowered = question.to_lowercase();
    let first_dataset = dataset_ids.first().copied();
    let first_ontology_type = ontology_type_ids.first().copied();

    let suggested_sql = if include_sql {
        if let Some(dataset_id) = first_dataset {
            Some(format!(
                "SELECT *\nFROM dataset_{}\nWHERE event_date >= CURRENT_DATE - INTERVAL '30 days'\nORDER BY event_date DESC\nLIMIT 100;",
                dataset_id.simple()
            ))
        } else if lowered.contains("sql") || lowered.contains("query") {
            Some(
				"SELECT *\nFROM your_dataset\nWHERE created_at >= CURRENT_DATE - INTERVAL '7 days';"
					.to_string(),
			)
        } else {
            None
        }
    } else {
        None
    };

    let pipeline_suggestions = if include_pipeline_plan {
        vec![
            "Profile the incoming source and verify schema drift before inference.".to_string(),
            "Materialize embeddings and retrieval chunks as a scheduled upstream step.".to_string(),
            "Add a guardrail validation node before publishing generated outputs.".to_string(),
        ]
    } else {
        Vec::new()
    };

    let mut ontology_hints = Vec::new();
    if let Some(object_type_id) = first_ontology_type {
        ontology_hints.push(format!(
            "Map the response to ontology type {} for downstream actioning.",
            object_type_id.simple()
        ));
    }
    if lowered.contains("ontology") || lowered.contains("object") {
        ontology_hints.push(
            "Prefer stable object identifiers and link types when grounding answers.".to_string(),
        );
    }

    let knowledge_summary = if knowledge_hits.is_empty() {
        "No indexed knowledge passages were required for this answer.".to_string()
    } else {
        format!(
            "Retrieved {} supporting passage(s), starting with '{}'.",
            knowledge_hits.len(),
            knowledge_hits[0].document_title
        )
    };

    CopilotDraft {
        answer: format!(
            "Copilot reviewed the request '{}'. {} Focus the next action on the most recent operational signal and keep the response ready for human verification.",
            truncate(question, 140),
            knowledge_summary
        ),
        suggested_sql,
        pipeline_suggestions,
        ontology_hints,
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
