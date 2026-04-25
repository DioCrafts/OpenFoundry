use std::cmp::Ordering;

use axum::http::{HeaderMap, header::AUTHORIZATION};
use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    domain::copilot,
    models::{
        agent::{AgentExecutionTrace, AgentPlanStep},
        knowledge_base::KnowledgeSearchResult,
        tool::ToolDefinition,
    },
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
            let output = execute_tool(
                client,
                tool,
                tool_name,
                user_message,
                objective,
                context,
                headers,
                knowledge_hits,
            )
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
                Some("skipped") => format!("Tool '{}' was skipped.", tool_name),
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
    knowledge_hits: &[KnowledgeSearchResult],
) -> Value {
    let Some(tool) = tool else {
        return json!({
            "tool": tool_name,
            "status": "failed",
            "error": "tool definition not found"
        });
    };

    let tool_inputs = resolve_tool_inputs(tool, tool_name, user_message, objective, context);
    if let Some(blocked) = enforce_tool_policy(tool, &tool_inputs, context) {
        return blocked;
    }

    match tool.execution_mode.as_str() {
        "simulated" => execute_simulated_tool(tool, &tool_inputs),
        "knowledge_search" => execute_knowledge_search_tool(tool, &tool_inputs, knowledge_hits),
        "native_sql" => {
            execute_native_sql_tool(tool, user_message, objective, &tool_inputs, knowledge_hits)
        }
        "native_dataset" => {
            execute_native_dataset_tool(tool, user_message, objective, &tool_inputs, knowledge_hits)
        }
        "native_ontology" => execute_native_ontology_tool(
            tool,
            user_message,
            objective,
            &tool_inputs,
            knowledge_hits,
        ),
        "native_pipeline" => {
            execute_native_pipeline_tool(tool, user_message, objective, &tool_inputs)
        }
        "native_report" => execute_native_report_tool(tool, user_message, objective, &tool_inputs),
        "native_workflow" => {
            execute_native_workflow_tool(tool, user_message, objective, &tool_inputs)
        }
        "native_code_repo" => {
            execute_native_code_repo_tool(tool, user_message, objective, &tool_inputs)
        }
        "http_json" => execute_http_tool(client, tool, &tool_inputs, headers, false).await,
        "openfoundry_api" => execute_http_tool(client, tool, &tool_inputs, headers, true).await,
        other => json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "skipped",
            "reason": format!("unsupported execution_mode '{}'", other)
        }),
    }
}

fn resolve_tool_inputs(
    tool: &ToolDefinition,
    tool_name: &str,
    user_message: &str,
    objective: &str,
    context: &Value,
) -> Value {
    context
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
                "question": user_message,
                "user_message": user_message,
                "objective": objective,
            })
        })
}

fn enforce_tool_policy(
    tool: &ToolDefinition,
    tool_inputs: &Value,
    context: &Value,
) -> Option<Value> {
    let sensitivity = tool
        .execution_config
        .get("sensitivity")
        .and_then(Value::as_str)
        .unwrap_or("normal");
    let requires_approval = tool
        .execution_config
        .get("requires_approval")
        .and_then(Value::as_bool)
        .unwrap_or(matches!(sensitivity, "high" | "mutating" | "admin"));
    if !requires_approval {
        return None;
    }

    let tool_policy = context.get("tool_policy");
    let allow_sensitive = tool_policy
        .and_then(|value| value.get("allow_sensitive_tools"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approved_tools = string_array(tool_policy.and_then(|value| value.get("approved_tools")));
    let tool_id = tool.id.to_string();
    let approved = allow_sensitive
        || approved_tools
            .iter()
            .any(|entry| entry == &tool.name || entry == &tool_id);

    if approved {
        return None;
    }

    Some(json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "blocked",
        "approval_required": true,
        "sensitivity": sensitivity,
        "required_approval_scope": tool.execution_config.get("approval_scope").and_then(Value::as_str).unwrap_or("operator"),
        "request": tool_inputs,
        "reason": format!("tool '{}' requires approval before execution", tool.name),
    }))
}

fn execute_simulated_tool(tool: &ToolDefinition, tool_inputs: &Value) -> Value {
    if let Some(mock_response) = tool.execution_config.get("mock_response") {
        json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "completed",
            "simulated": true,
            "request": tool_inputs,
            "response": mock_response,
        })
    } else {
        json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "skipped",
            "simulated": true,
            "reason": "simulated tool has no mock_response configured"
        })
    }
}

fn execute_knowledge_search_tool(
    tool: &ToolDefinition,
    tool_inputs: &Value,
    knowledge_hits: &[KnowledgeSearchResult],
) -> Value {
    let query = tool_inputs
        .get("query")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            tool_inputs
                .get("question")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("")
        });
    let top_k = usize_from_value(tool_inputs.get("top_k"))
        .or_else(|| usize_from_value(tool.execution_config.get("top_k")))
        .unwrap_or(4)
        .max(1);
    let min_score = f32_from_value(tool_inputs.get("min_score"))
        .or_else(|| f32_from_value(tool.execution_config.get("min_score")))
        .unwrap_or(0.15);

    let mut ranked_hits = knowledge_hits
        .iter()
        .map(|hit| {
            let lexical = lexical_score(query, &format!("{} {}", hit.document_title, hit.excerpt));
            let score = if query.trim().is_empty() {
                hit.score
            } else {
                ((hit.score * 0.6) + (lexical * 0.4)).min(1.0)
            };

            json!({
                "document_title": hit.document_title,
                "score": score,
                "retrieval_score": hit.score,
                "excerpt": hit.excerpt,
                "source_uri": hit.source_uri,
                "metadata": hit.metadata,
            })
        })
        .filter(|hit| {
            hit.get("score")
                .and_then(Value::as_f64)
                .map(|score| score as f32 >= min_score)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    ranked_hits.sort_by(|left, right| {
        let left_score = left.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let right_score = right.get("score").and_then(Value::as_f64).unwrap_or(0.0);

        right_score
            .partial_cmp(&left_score)
            .unwrap_or(Ordering::Equal)
    });
    ranked_hits.truncate(top_k);

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "query": query,
        "result_count": ranked_hits.len(),
        "results": ranked_hits,
    })
}

fn execute_native_sql_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
    knowledge_hits: &[KnowledgeSearchResult],
) -> Value {
    let question = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let dataset_ids = extract_uuid_array(tool_inputs.get("dataset_ids"));
    let draft = copilot::assist(question, &dataset_ids, &[], knowledge_hits, true, false);

    let table_name = tool_inputs
        .get("dataset_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_dataset_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("your_dataset");
    let time_column = tool_inputs
        .get("time_column")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("time_column")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or(if question.to_lowercase().contains("event") {
            "event_date"
        } else {
            "created_at"
        });
    let limit = usize_from_value(tool_inputs.get("limit"))
        .or_else(|| usize_from_value(tool.execution_config.get("default_limit")))
        .unwrap_or(100)
        .max(1);
    let lookback_days = infer_time_window_days(question);
    let metric_hints = string_array(tool.execution_config.get("metric_hints"))
        .into_iter()
        .chain(string_array(tool_inputs.get("metric_hints")))
        .collect::<Vec<_>>();

    let generated_sql = draft.suggested_sql.unwrap_or_else(|| {
        let order_column = if metric_hints
            .iter()
            .any(|hint| hint.contains("latency") || hint.contains("error"))
        {
            metric_hints
                .first()
                .cloned()
                .unwrap_or_else(|| time_column.to_string())
        } else {
            time_column.to_string()
        };

        format!(
            "SELECT *\nFROM {table_name}\nWHERE {time_column} >= CURRENT_DATE - INTERVAL '{lookback_days} days'\nORDER BY {order_column} DESC\nLIMIT {limit};"
        )
    });

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "sql": generated_sql,
        "dataset_name": table_name,
        "lookback_days": lookback_days,
        "limit": limit,
        "objective": objective,
        "explanation": format!(
            "Generated starter SQL for '{}' using '{}' as the working dataset.",
            question,
            table_name
        ),
    })
}

fn execute_native_ontology_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
    knowledge_hits: &[KnowledgeSearchResult],
) -> Value {
    let answer = tool_inputs
        .get("answer")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let ontology_type_ids = extract_uuid_array(tool_inputs.get("ontology_type_ids"));
    let draft = copilot::assist(
        answer,
        &[],
        &ontology_type_ids,
        knowledge_hits,
        false,
        false,
    );

    let mut object_types = string_array(tool.execution_config.get("default_object_types"));
    object_types.extend(infer_object_types(answer));
    object_types.sort();
    object_types.dedup();

    let link_type = tool
        .execution_config
        .get("default_link_type")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("RELATED_TO");

    let actions = infer_action_hints(answer, objective);
    let links = if object_types.len() >= 2 {
        vec![format!(
            "{} --{}--> {}",
            object_types[0], link_type, object_types[1]
        )]
    } else {
        Vec::new()
    };

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "object_types": object_types,
        "link_suggestions": links,
        "ontology_hints": draft.ontology_hints,
        "recommended_actions": actions,
    })
}

fn execute_native_dataset_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
    knowledge_hits: &[KnowledgeSearchResult],
) -> Value {
    let question = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let dataset_ids = extract_uuid_array(tool_inputs.get("dataset_ids"));
    let draft = copilot::assist(question, &dataset_ids, &[], knowledge_hits, true, false);
    let dataset_name = tool_inputs
        .get("dataset_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_dataset_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("operational_dataset");
    let operation = infer_dataset_operation(question, objective);
    let branch_name = format!(
        "{}-{}",
        tool.execution_config
            .get("branch_prefix")
            .and_then(Value::as_str)
            .unwrap_or("analysis"),
        dataset_name.replace('_', "-")
    );
    let governance = infer_dataset_governance(question, objective);

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "dataset_name": dataset_name,
        "dataset_ids": dataset_ids,
        "operation": operation,
        "recommended_branch": branch_name,
        "governance_checks": governance,
        "suggested_sql": draft.suggested_sql,
        "next_actions": [
            format!("Preview rows from '{}'.", dataset_name),
            format!("Run dataset linter before mutating '{}'.", dataset_name),
            format!("Open a branch and stage changes for '{}'.", dataset_name),
        ],
    })
}

fn execute_native_pipeline_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
) -> Value {
    let question = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let pipeline_name = tool_inputs
        .get("pipeline_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_pipeline_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("platform_pipeline");
    let run_mode = if question.to_lowercase().contains("full rebuild")
        || objective.to_lowercase().contains("full rebuild")
    {
        "full_rebuild"
    } else {
        "incremental"
    };
    let trigger = infer_pipeline_trigger(question, objective);

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "pipeline_name": pipeline_name,
        "run_mode": run_mode,
        "trigger_reason": trigger,
        "recommended_inputs": string_array(tool.execution_config.get("input_datasets")),
        "recommended_outputs": string_array(tool.execution_config.get("output_datasets")),
        "next_actions": [
            format!("Inspect latest run for '{}'.", pipeline_name),
            format!("Trigger {} execution for '{}'.", run_mode, pipeline_name),
            "Review downstream lineage impact before promotion.".to_string(),
        ],
    })
}

fn execute_native_report_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
) -> Value {
    let prompt = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let report_name = tool_inputs
        .get("report_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_report_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("operations_digest");
    let channels = infer_report_channels(
        prompt,
        &string_array(tool.execution_config.get("default_channels")),
    );

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "report_name": report_name,
        "distribution_channels": channels,
        "schedule_hint": infer_report_schedule(prompt, objective),
        "delivery_actions": [
            format!("Generate preview for '{}'.", report_name),
            format!("Dispatch '{}' through the selected channels.", report_name),
            "Archive the execution manifest in object storage.".to_string(),
        ],
    })
}

fn execute_native_workflow_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
) -> Value {
    let prompt = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let workflow_name = tool_inputs
        .get("workflow_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_workflow_name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("operator_review");

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "workflow_name": workflow_name,
        "proposal_type": infer_workflow_proposal(prompt, objective),
        "approval_mode": tool.execution_config.get("approval_scope").and_then(Value::as_str).unwrap_or("operator"),
        "steps": [
            "Assemble action proposal".to_string(),
            "Request human approval".to_string(),
            "Execute submit_action or notification step after approval".to_string(),
        ],
    })
}

fn execute_native_code_repo_tool(
    tool: &ToolDefinition,
    user_message: &str,
    objective: &str,
    tool_inputs: &Value,
) -> Value {
    let prompt = tool_inputs
        .get("question")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(user_message);
    let repository = tool_inputs
        .get("repository")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            tool.execution_config
                .get("default_repository")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or("openfoundry-platform");
    let branch = format!(
        "{}/{}",
        tool.execution_config
            .get("branch_prefix")
            .and_then(Value::as_str)
            .unwrap_or("agent"),
        infer_repo_branch_slug(prompt, objective)
    );

    json!({
        "tool": tool.name,
        "category": tool.category,
        "status": "completed",
        "repository": repository,
        "branch": branch,
        "merge_request_title": infer_repo_mr_title(prompt, objective),
        "required_checks": string_array(tool.execution_config.get("required_checks")),
        "next_actions": [
            format!("Create or update branch '{}'.", branch),
            "Run required CI checks before merge.".to_string(),
            "Open a merge request with operator-facing summary.".to_string(),
        ],
    })
}

async fn execute_http_tool(
    client: &reqwest::Client,
    tool: &ToolDefinition,
    tool_inputs: &Value,
    headers: &HeaderMap,
    platform_mode: bool,
) -> Value {
    let Some(url) = resolve_http_url(tool, tool_inputs, platform_mode) else {
        return json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "failed",
            "error": if platform_mode {
                "missing execution_config.path or execution_config.url/base_url for openfoundry_api"
            } else {
                "missing execution_config.url"
            }
        });
    };

    let method = tool
        .execution_config
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("POST")
        .to_uppercase();
    let method = Method::from_bytes(method.as_bytes()).unwrap_or(Method::POST);

    let auth_mode = if platform_mode {
        tool.execution_config
            .get("auth_mode")
            .and_then(Value::as_str)
            .unwrap_or("forward_bearer")
    } else {
        tool.execution_config
            .get("auth_mode")
            .and_then(Value::as_str)
            .unwrap_or("none")
    };

    let mut request = client.request(method.clone(), url.clone());
    if auth_mode == "forward_bearer" {
        if let Some(value) = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
        {
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
        request = request.json(tool_inputs);
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
                    "url": url,
                }),
                Ok(payload) => json!({
                    "tool": tool.name,
                    "category": tool.category,
                    "status": "failed",
                    "http_status": status.as_u16(),
                    "request": tool_inputs,
                    "response": payload,
                    "url": url,
                }),
                Err(cause) => json!({
                    "tool": tool.name,
                    "category": tool.category,
                    "status": "failed",
                    "http_status": status.as_u16(),
                    "error": format!("failed to parse tool response: {cause}"),
                    "url": url,
                }),
            }
        }
        Err(cause) => json!({
            "tool": tool.name,
            "category": tool.category,
            "status": "failed",
            "error": format!("tool request failed: {cause}"),
            "url": url,
        }),
    }
}

fn resolve_http_url(
    tool: &ToolDefinition,
    tool_inputs: &Value,
    platform_mode: bool,
) -> Option<String> {
    let url = tool
        .execution_config
        .get("url")
        .and_then(Value::as_str)
        .map(|value| render_template(value, tool_inputs))
        .filter(|value| !value.trim().is_empty());
    if url.is_some() {
        return url;
    }

    if !platform_mode {
        return None;
    }

    let path = tool
        .execution_config
        .get("path")
        .or_else(|| tool.execution_config.get("path_template"))
        .and_then(Value::as_str)
        .map(|value| render_template(value, tool_inputs))
        .filter(|value| !value.trim().is_empty())?;
    if path.starts_with("http://") || path.starts_with("https://") {
        return Some(path);
    }
    if !path.starts_with("/api/") {
        return None;
    }

    let base_url = tool
        .execution_config
        .get("base_url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| std::env::var("OPENFOUNDRY_API_BASE_URL").ok())?;

    Some(format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    ))
}

fn infer_time_window_days(question: &str) -> usize {
    let lowered = question.to_lowercase();
    if lowered.contains("90 day") || lowered.contains("90-day") {
        90
    } else if lowered.contains("30 day") || lowered.contains("30-day") || lowered.contains("month")
    {
        30
    } else if lowered.contains("24 hour") || lowered.contains("today") {
        1
    } else {
        7
    }
}

fn infer_object_types(content: &str) -> Vec<String> {
    let lowered = content.to_lowercase();
    let mut types = Vec::new();

    if lowered.contains("incident") || lowered.contains("alert") {
        types.push("Incident".to_string());
    }
    if lowered.contains("provider") || lowered.contains("model") {
        types.push("Provider".to_string());
    }
    if lowered.contains("dataset") || lowered.contains("table") {
        types.push("Dataset".to_string());
    }
    if lowered.contains("workflow") || lowered.contains("approval") {
        types.push("Workflow".to_string());
    }
    if lowered.contains("pipeline") || lowered.contains("build") {
        types.push("Pipeline".to_string());
    }

    if types.is_empty() {
        types.push("OperationalObject".to_string());
    }

    types
}

fn infer_action_hints(content: &str, objective: &str) -> Vec<String> {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    let mut actions = Vec::new();

    if lowered.contains("reroute") || lowered.contains("fallback") {
        actions.push("Submit reroute action for the affected provider.".to_string());
    }
    if lowered.contains("notify") || lowered.contains("alert") {
        actions.push("Notify operators and attach the generated context.".to_string());
    }
    if lowered.contains("approve") || lowered.contains("review") {
        actions.push("Open approval workflow with the proposed change set.".to_string());
    }
    if actions.is_empty() {
        actions.push("Prepare a human-reviewable action proposal.".to_string());
    }

    actions
}

fn infer_dataset_operation(content: &str, objective: &str) -> &'static str {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    if lowered.contains("lint") || lowered.contains("quality") {
        "lint_and_quality_review"
    } else if lowered.contains("branch") || lowered.contains("what-if") {
        "branch_and_preview"
    } else if lowered.contains("export") {
        "export_and_share"
    } else {
        "preview_and_investigate"
    }
}

fn infer_dataset_governance(content: &str, objective: &str) -> Vec<String> {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    let mut checks = vec!["lineage impact review".to_string()];
    if lowered.contains("pii") || lowered.contains("sensitive") {
        checks.push("marking and restricted-view validation".to_string());
    }
    if lowered.contains("export") || lowered.contains("share") {
        checks.push("delivery recipient review".to_string());
    }
    checks
}

fn infer_pipeline_trigger(content: &str, objective: &str) -> String {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    if lowered.contains("incident") || lowered.contains("failed") {
        "recover_failed_run".to_string()
    } else if lowered.contains("backfill") {
        "historical_backfill".to_string()
    } else {
        "operator_requested_refresh".to_string()
    }
}

fn infer_report_channels(content: &str, defaults: &[String]) -> Vec<String> {
    let lowered = content.to_lowercase();
    let mut channels = defaults.to_vec();
    if lowered.contains("slack") {
        channels.push("slack".to_string());
    }
    if lowered.contains("teams") {
        channels.push("teams".to_string());
    }
    if lowered.contains("email") || channels.is_empty() {
        channels.push("email".to_string());
    }
    if lowered.contains("webhook") {
        channels.push("webhook".to_string());
    }
    channels.sort();
    channels.dedup();
    channels
}

fn infer_report_schedule(content: &str, objective: &str) -> &'static str {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    if lowered.contains("daily") {
        "daily"
    } else if lowered.contains("weekly") {
        "weekly"
    } else if lowered.contains("incident") || lowered.contains("on-demand") {
        "manual"
    } else {
        "cron"
    }
}

fn infer_workflow_proposal(content: &str, objective: &str) -> &'static str {
    let lowered = format!("{} {}", content.to_lowercase(), objective.to_lowercase());
    if lowered.contains("submit action") || lowered.contains("mutate") {
        "submit_action"
    } else if lowered.contains("approval") || lowered.contains("review") {
        "approval_review"
    } else {
        "notification_orchestration"
    }
}

fn infer_repo_branch_slug(content: &str, objective: &str) -> String {
    let combined = format!("{} {}", content, objective).to_lowercase();
    let mut slug = combined
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "agent-change".to_string()
    } else {
        slug.chars().take(48).collect()
    }
}

fn infer_repo_mr_title(content: &str, objective: &str) -> String {
    let prompt = content.trim();
    if prompt.is_empty() {
        format!("Agent proposal: {}", objective.trim())
    } else {
        format!("Agent proposal: {}", prompt)
    }
}

fn lexical_score(query: &str, haystack: &str) -> f32 {
    let normalized_query = query.to_lowercase();
    let query_terms = normalized_query
        .split_whitespace()
        .filter(|term| term.len() > 2)
        .collect::<Vec<_>>();
    if query_terms.is_empty() {
        return 0.0;
    }

    let haystack = haystack.to_lowercase();
    let hits = query_terms
        .iter()
        .filter(|term| haystack.contains(**term))
        .count();

    hits as f32 / query_terms.len() as f32
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn usize_from_value(value: Option<&Value>) -> Option<usize> {
    value
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn f32_from_value(value: Option<&Value>) -> Option<f32> {
    value.and_then(Value::as_f64).map(|value| value as f32)
}

fn extract_uuid_array(value: Option<&Value>) -> Vec<Uuid> {
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|entry| Uuid::parse_str(entry).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn render_template(template: &str, values: &Value) -> String {
    let mut rendered = template.to_string();
    if let Some(values) = values.as_object() {
        for (key, value) in values {
            rendered = rendered.replace(&format!("{{{key}}}"), &query_value(value));
        }
    }
    rendered
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
    use chrono::Utc;
    use serde_json::{Value, json};
    use uuid::Uuid;

    use crate::models::{
        agent::AgentPlanStep, knowledge_base::KnowledgeSearchResult, tool::ToolDefinition,
    };

    use super::execute_plan;

    fn tool(name: &str, execution_mode: &str, execution_config: Value) -> ToolDefinition {
        ToolDefinition {
            id: Uuid::now_v7(),
            name: name.to_string(),
            description: format!("{name} description"),
            category: "analysis".to_string(),
            execution_mode: execution_mode.to_string(),
            execution_config,
            status: "active".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn simulated_tools_return_mock_response() {
        let plan = vec![AgentPlanStep {
            id: "tool-mock".to_string(),
            title: "Invoke tool".to_string(),
            description: String::new(),
            tool_name: Some("Mock Tool".to_string()),
            status: "planned".to_string(),
        }];
        let traces = execute_plan(
            &reqwest::Client::new(),
            &plan,
            &[tool(
                "Mock Tool",
                "simulated",
                json!({ "mock_response": { "ok": true } }),
            )],
            "score this account",
            "predict churn risk",
            &json!({}),
            &HeaderMap::new(),
            &[],
        )
        .await;

        assert_eq!(traces[0].output["status"], "completed");
        assert_eq!(traces[0].output["response"]["ok"], true);
    }

    #[tokio::test]
    async fn executes_native_sql_tools() {
        let plan = vec![AgentPlanStep {
            id: "tool-sql".to_string(),
            title: "Invoke SQL tool".to_string(),
            description: String::new(),
            tool_name: Some("SQL Generator".to_string()),
            status: "planned".to_string(),
        }];
        let traces = execute_plan(
            &reqwest::Client::new(),
            &plan,
            &[tool(
                "SQL Generator",
                "native_sql",
                json!({
                    "default_dataset_name": "provider_metrics",
                    "time_column": "event_date",
                    "default_limit": 50
                }),
            )],
            "Show the highest latency providers in the last 30 days",
            "Investigate provider latency",
            &json!({}),
            &HeaderMap::new(),
            &[],
        )
        .await;

        assert_eq!(traces[0].output["status"], "completed");
        assert!(
            traces[0].output["sql"]
                .as_str()
                .unwrap()
                .contains("provider_metrics")
        );
    }

    #[tokio::test]
    async fn executes_native_dataset_tools() {
        let plan = vec![AgentPlanStep {
            id: "tool-dataset".to_string(),
            title: "Invoke dataset tool".to_string(),
            description: String::new(),
            tool_name: Some("Dataset Navigator".to_string()),
            status: "planned".to_string(),
        }];
        let traces = execute_plan(
            &reqwest::Client::new(),
            &plan,
            &[tool(
                "Dataset Navigator",
                "native_dataset",
                json!({
                    "default_dataset_name": "customer_health",
                    "branch_prefix": "what-if"
                }),
            )],
            "Open a what-if branch and lint the customer health dataset",
            "Investigate churn signal drift",
            &json!({}),
            &HeaderMap::new(),
            &[],
        )
        .await;

        assert_eq!(traces[0].output["status"], "completed");
        assert_eq!(traces[0].output["dataset_name"], "customer_health");
        assert_eq!(traces[0].output["operation"], "lint_and_quality_review");
    }

    #[tokio::test]
    async fn blocks_sensitive_tools_without_approval() {
        let plan = vec![AgentPlanStep {
            id: "tool-workflow".to_string(),
            title: "Invoke workflow tool".to_string(),
            description: String::new(),
            tool_name: Some("Workflow Operator".to_string()),
            status: "planned".to_string(),
        }];
        let traces = execute_plan(
            &reqwest::Client::new(),
            &plan,
            &[tool(
                "Workflow Operator",
                "native_workflow",
                json!({
                    "requires_approval": true,
                    "approval_scope": "operator",
                    "sensitivity": "mutating"
                }),
            )],
            "Submit action to reroute providers",
            "Mutate production workflow",
            &json!({}),
            &HeaderMap::new(),
            &[],
        )
        .await;

        assert_eq!(traces[0].output["status"], "blocked");
        assert_eq!(traces[0].output["approval_required"], true);
    }

    #[tokio::test]
    async fn executes_knowledge_search_tools() {
        let plan = vec![AgentPlanStep {
            id: "tool-search".to_string(),
            title: "Invoke search tool".to_string(),
            description: String::new(),
            tool_name: Some("Knowledge Search".to_string()),
            status: "planned".to_string(),
        }];
        let knowledge_hits = vec![KnowledgeSearchResult {
            knowledge_base_id: Uuid::now_v7(),
            document_id: Uuid::now_v7(),
            document_title: "Incident Playbook".to_string(),
            chunk_id: "chunk-1".to_string(),
            score: 0.91,
            excerpt: "Reroute providers when latency exceeds 500ms for three checks.".to_string(),
            source_uri: Some("kb://incident-playbook".to_string()),
            metadata: json!({}),
        }];

        let traces = execute_plan(
            &reqwest::Client::new(),
            &plan,
            &[tool(
                "Knowledge Search",
                "knowledge_search",
                json!({ "top_k": 2 }),
            )],
            "How do I reroute an overloaded provider?",
            "Investigate provider latency",
            &json!({}),
            &HeaderMap::new(),
            &knowledge_hits,
        )
        .await;

        assert_eq!(traces[0].output["status"], "completed");
        assert_eq!(traces[0].output["result_count"], 1);
    }
}
