use crate::models::{
    agent::{AgentDefinition, AgentPlanStep},
    knowledge_base::KnowledgeSearchResult,
    tool::ToolDefinition,
};

pub fn build_plan(
    agent: &AgentDefinition,
    objective: &str,
    tools: &[ToolDefinition],
    knowledge_hits: &[KnowledgeSearchResult],
) -> Vec<AgentPlanStep> {
    let mut steps = vec![AgentPlanStep {
        id: "analyze-request".to_string(),
        title: "Analyze user intent".to_string(),
        description: format!("Align the request with agent objective '{}'.", objective),
        tool_name: None,
        status: "planned".to_string(),
    }];

    if !knowledge_hits.is_empty() {
        steps.push(AgentPlanStep {
            id: "retrieve-context".to_string(),
            title: "Retrieve supporting context".to_string(),
            description: format!(
                "Use {} retrieved passage(s) before drafting the answer.",
                knowledge_hits.len()
            ),
            tool_name: None,
            status: "planned".to_string(),
        });
    }

    for tool in tools.iter().take(agent.max_iterations.max(1) as usize) {
        steps.push(AgentPlanStep {
            id: format!("tool-{}", tool.name.to_lowercase().replace(' ', "-")),
            title: format!("Invoke {}", tool.name),
            description: tool.description.clone(),
            tool_name: Some(tool.name.clone()),
            status: "planned".to_string(),
        });
    }

    steps.push(AgentPlanStep {
        id: "synthesize-answer".to_string(),
        title: "Synthesize final answer".to_string(),
        description: format!(
            "Use planning strategy '{}' and produce an operator-friendly summary.",
            agent.planning_strategy
        ),
        tool_name: None,
        status: "planned".to_string(),
    });

    steps
}
