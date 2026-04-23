use serde_json::json;

use crate::models::{
	agent::{AgentExecutionTrace, AgentPlanStep},
	knowledge_base::KnowledgeSearchResult,
	tool::ToolDefinition,
};

pub fn execute_plan(
	plan: &[AgentPlanStep],
	tools: &[ToolDefinition],
	user_message: &str,
	knowledge_hits: &[KnowledgeSearchResult],
) -> Vec<AgentExecutionTrace> {
	plan
		.iter()
		.map(|step| {
			let output = if let Some(tool_name) = &step.tool_name {
				let tool = tools.iter().find(|candidate| candidate.name == *tool_name);
				json!({
					"tool": tool_name,
					"category": tool.map(|value| value.category.clone()).unwrap_or_else(|| "generic".to_string()),
					"status": "simulated",
					"message": format!("Executed '{}' against '{}'.", tool_name, user_message),
				})
			} else if step.id == "retrieve-context" {
				json!({
					"citations": knowledge_hits.iter().map(|hit| hit.document_title.clone()).collect::<Vec<_>>()
				})
			} else {
				json!({ "status": "completed" })
			};

			let observation = if step.id == "synthesize-answer" {
				format!(
					"Prepared a final answer using {} tool invocation(s) and {} knowledge hit(s).",
					tools.len(),
					knowledge_hits.len()
				)
			} else {
				format!("Completed plan step '{}'.", step.title)
			};

			AgentExecutionTrace {
				step_id: step.id.clone(),
				title: step.title.clone(),
				tool_name: step.tool_name.clone(),
				observation,
				output,
			}
		})
		.collect()
}
