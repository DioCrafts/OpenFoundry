use std::{collections::HashMap, sync::Arc};

use auth_middleware::{Claims, JwtConfig, jwt::encode_token};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::kernel::{KernelExecutionContext, KernelExecutionResult};

pub type LlmSessions = Arc<RwLock<HashMap<Uuid, Uuid>>>;

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    conversation_id: Option<Uuid>,
    user_message: &'a str,
    system_prompt: String,
    fallback_enabled: bool,
    max_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct UsageSummary {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    estimated_cost_usd: f32,
    latency_ms: i32,
    network_scope: String,
    cache_hit: bool,
}

#[derive(Debug, Deserialize)]
struct Citation {
    document_title: String,
    excerpt: String,
    source_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    conversation_id: Uuid,
    provider_name: String,
    reply: String,
    citations: Vec<Citation>,
    usage: UsageSummary,
    created_at: String,
}

pub async fn ensure_session(_sessions: &LlmSessions, _session_id: Uuid) -> Result<(), String> {
    Ok(())
}

pub async fn drop_session(sessions: &LlmSessions, session_id: Uuid) {
    sessions.write().await.remove(&session_id);
}

pub async fn execute(
    sessions: &LlmSessions,
    http_client: &Client,
    ai_service_url: &str,
    jwt_config: &JwtConfig,
    claims: &Claims,
    source: &str,
    session_id: Option<Uuid>,
    context: &KernelExecutionContext,
) -> Result<KernelExecutionResult, String> {
    let token = encode_token(jwt_config, claims)
        .map_err(|error| format!("failed to sign AI service token: {error}"))?;

    let conversation_id = if let Some(session_id) = session_id {
        sessions.read().await.get(&session_id).copied()
    } else {
        None
    };

    let response = http_client
        .post(format!(
            "{}/api/v1/ai/chat/completions",
            ai_service_url.trim_end_matches('/'),
        ))
        .bearer_auth(token)
        .json(&ChatCompletionRequest {
            conversation_id,
            user_message: source,
            system_prompt: build_system_prompt(context),
            fallback_enabled: true,
            max_tokens: 900,
        })
        .send()
        .await
        .map_err(|error| format!("ai-service request failed: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let error_payload: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({ "error": status.to_string() }));
        let error = error_payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("LLM completion failed");
        return Err(error.to_string());
    }

    let payload: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|error| format!("invalid ai-service response: {error}"))?;

    if let Some(session_id) = session_id {
        sessions
            .write()
            .await
            .insert(session_id, payload.conversation_id);
    }

    Ok(KernelExecutionResult {
        output_type: "llm".to_string(),
        content: serde_json::json!({
            "reply": payload.reply,
            "provider_name": payload.provider_name,
            "conversation_id": payload.conversation_id,
            "citations": payload.citations.iter().map(|citation| serde_json::json!({
                "document_title": citation.document_title,
                "excerpt": citation.excerpt,
                "source_uri": citation.source_uri,
            })).collect::<Vec<_>>(),
            "usage": {
                "prompt_tokens": payload.usage.prompt_tokens,
                "completion_tokens": payload.usage.completion_tokens,
                "total_tokens": payload.usage.total_tokens,
                "estimated_cost_usd": payload.usage.estimated_cost_usd,
                "latency_ms": payload.usage.latency_ms,
                "network_scope": payload.usage.network_scope,
                "cache_hit": payload.usage.cache_hit,
            },
            "created_at": payload.created_at,
        }),
    })
}

fn build_system_prompt(context: &KernelExecutionContext) -> String {
    let mut prompt = format!(
        "You are assisting inside an OpenFoundry notebook cell.\nNotebook ID: {}\n",
        context.notebook_id
    );

    if let Some(workspace_dir) = &context.workspace_dir {
        prompt.push_str(&format!("Workspace directory: {workspace_dir}\n"));
    }

    if !context.workspace_files.is_empty() {
        prompt.push_str("Workspace files in scope:\n");
        for file in context.workspace_files.iter().take(5) {
            let snippet = truncate(&file.content, 900);
            prompt.push_str(&format!("--- {} ---\n{}\n", file.path, snippet));
        }
    }

    prompt.push_str(
        "\nBe concise, technical, and notebook-friendly. When the user asks for code, return runnable code or precise operational guidance.",
    );
    prompt
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>() + "\n...[truncated]"
}
