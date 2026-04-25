use axum::http::{HeaderMap, header::AUTHORIZATION};
use reqwest::Method;
use serde_json::{Value, json};

use crate::models::{
    agent::{AgentExecutionTrace, AgentPlanStep},
    knowledge_base::KnowledgeSearchResult,
    tool::ToolDefinition,
};

pub async fn execute_plan(
    client: &reqwest::Client,
    plan: &[AgentPlanStep],
    tools: &[ToolDefinition],
    user_message: &str,
    objective: &str,
    context: &Value,
    headers: &HeaderMap,
    knowledge_hits: &[KnowledgeSearchResult],
) -> Vec<AgentExecutionTrace> {
    let mut traces = Vec::with_capacity(plan.len());
    let mut successful_tool_invocations = 0usize;

    for step in plan {
        let (output, observation) = if let Some(tool_name) = &step.tool_name {
            let tool = tools.iter().find(|candidate| candidate.name == *tool_name);
            let output =
                execute_tool(client, tool, tool_name, user_message, objective, context, headers)
                    .await;
            if output
                .get("status")
                .and_then(Value::as_str)
                .map(|status| status == "completed")
                .unwrap_or(false)
            {
                successful_tool_invocations += 1;
            }

            let observation = match output.get("status").and_then(Value::as_str) {
                Some("completed") => format!("Executed tool '{}'.", tool_name),
                Some("failed") => format!("Tool '{}' failed.", tool_name),
                Some(other) => format!("Tool '{}' finished with status '{}'.", tool_name, other),
                None => format!("Tool '{}' produced an unstructured response.", tool_name),
            };

            (output, observation)
        } else if step.id == "retrieve-context" {
            (
                json!({
                    "citations": knowledge_hits.iter().map(|hit| {
                        json!({
                            "document_title": hit.document_title,
                            "score": hit.score,
                            "excerpt": hit.excerpt,
                        })
                    }).collect::<Vec<_>>()
                }),
                format!("Retrieved {} knowledge hit(s).", knowledge_hits.len()),
            )
        } else if step.id == "synthesize-answer" {
            (
                json!({ "status": "completed" }),
                format!(
                    "Prepared final synthesis with {} successful tool invocation(s) and {} knowledge hit(s).",
                    successful_tool_invocations,
                    knowledge_hits.len()
                ),
            )
        } else {
            (
                json!({ "status": "completed" }),
                format!("Completed plan step '{}'.", step.title),
            )
        };

        traces.push(AgentExecutionTrace {
            step_id: step.id.clone(),
            title: step.title.clone(),
            tool_name: step.tool_name.clone(),
            observation,
            output,
        });
    }

    traces
}

async fn execute_tool(
    client: &reqwest::Client,
    tool: Option<&ToolDefinition>,
    tool_name: &str,
    user_message: &str,
    objective: &str,
    context: &Value,
    headers: &HeaderMap,
) -> Value {
    let Some(tool) = tool else {
        return json!({
            "tool": tool_name,
            "status": "failed",
            "error": "tool definition not found"
        });
    };

    if tool.execution_mode != "http_json" {
        return json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "skipped",
            "reason": format!("unsupported execution_mode '{}'", tool.execution_mode)
        });
    }

    let Some(url) = tool
        .execution_config
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "failed",
            "error": "missing execution_config.url"
        });
    };

    let method = tool
        .execution_config
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("POST")
        .to_uppercase();
    let method = Method::from_bytes(method.as_bytes()).unwrap_or(Method::POST);

    let auth_mode = tool
        .execution_config
        .get("auth_mode")
        .and_then(Value::as_str)
        .unwrap_or("none");

    let tool_inputs = context
        .get("tool_inputs")
        .and_then(|value| value.get(tool_name))
        .cloned()
        .or_else(|| {
            let tool_id = tool.id.to_string();
            context
                .get("tool_inputs")
                .and_then(|value| value.get(tool_id.as_str()))
                .cloned()
        })
        .unwrap_or_else(|| {
            json!({
                "user_message": user_message,
                "objective": objective,
            })
        });

    let mut request = client.request(method.clone(), url);
    if auth_mode == "forward_bearer" {
        if let Some(value) = headers.get(AUTHORIZATION).and_then(|value| value.to_str().ok()) {
            request = request.header(AUTHORIZATION, value);
        }
    }

    if let Some(extra_headers) = tool
        .execution_config
        .get("headers")
        .and_then(Value::as_object)
        .cloned()
    {
        for (key, value) in extra_headers {
            if let Some(value) = value.as_str() {
                request = request.header(key, value);
            }
        }
    }

    if method == Method::GET {
        if let Some(query) = tool_inputs.as_object() {
            let query_pairs = query
                .iter()
                .map(|(key, value)| (key.clone(), query_value(value)))
                .collect::<Vec<_>>();
            request = request.query(&query_pairs);
        }
    } else {
        request = request.json(&tool_inputs);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            match response.json::<Value>().await {
                Ok(payload) if status.is_success() => json!({
                    "tool": tool.name,
                    "category": tool.category,
                    "status": "completed",
                    "http_status": status.as_u16(),
                    "request": tool_inputs,
                    "response": payload,
                }),
                Ok(payload) => json!({
                    "tool": tool.name,
                    "category": tool.category,
                    "status": "failed",
                    "http_status": status.as_u16(),
                    "request": tool_inputs,
                    "response": payload,
                }),
                Err(cause) => json!({
                    "tool": tool.name,
                    "category": tool.category,
                    "status": "failed",
                    "http_status": status.as_u16(),
                    "error": format!("failed to parse tool response: {cause}"),
                }),
            }
        }
        Err(cause) => json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "failed",
            "error": format!("tool request failed: {cause}"),
        }),
    }
}

fn query_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::{agent::AgentPlanStep, tool::ToolDefinition};

    use super::execute_plan;

    #[tokio::test]
    async fn skips_unsupported_tool_modes_without_network_calls() {
        let tool = ToolDefinition {
            id: Uuid::now_v7(),
            name: "Risk Predictor".to_string(),
            description: "Scores risk via HTTP".to_string(),
            category: "ml".to_string(),
            execution_mode: "simulated".to_string(),
            execution_config: json!({}),
            status: "active".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let plan = vec![AgentPlanStep {
            id: "tool-risk-predictor".to_string(),
            title: "Invoke tool".to_string(),
            description: String::new(),
            tool_name: Some(tool.name.clone()),
            status: "planned".to_string(),
        }];
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer smoke-token".parse().unwrap());

        let traces = execute_plan(
            &client,
            &plan,
            &[tool],
            "score this account",
            "predict churn risk",
            &json!({
                "tool_inputs": {
                    "Risk Predictor": {
                        "inputs": [{ "tickets_open": 9, "usage_delta": -0.72 }]
                    }
                }
            }),
            &headers,
            &[],
        )
        .await;

        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].output["status"], "skipped");
    }
}
