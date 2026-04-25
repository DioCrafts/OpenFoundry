use serde_json::{Value, json};

use crate::models::provider::LlmProvider;

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub text: String,
}

pub async fn complete_text(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f32,
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    match provider.api_mode.as_str() {
        "chat_completions" => complete_openai_compatible(
            client,
            provider,
            system_prompt,
            user_prompt,
            temperature,
            max_tokens,
        )
        .await,
        "messages" => {
            complete_anthropic(client, provider, system_prompt, user_prompt, max_tokens).await
        }
        "chat" => complete_ollama(client, provider, system_prompt, user_prompt).await,
        mode => Err(format!("unsupported provider api_mode '{mode}'")),
    }
}

pub async fn embed_text(
    client: &reqwest::Client,
    provider: &LlmProvider,
    content: &str,
) -> Result<Vec<f32>, String> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    match provider.api_mode.as_str() {
        "chat_completions" => embed_openai_compatible(client, provider, content).await,
        "chat" => embed_ollama(client, provider, content).await,
        mode => Err(format!(
            "provider api_mode '{mode}' does not support embeddings in ai-service"
        )),
    }
}

fn provider_token(provider: &LlmProvider) -> Option<String> {
    provider
        .credential_reference
        .as_deref()
        .and_then(|reference| std::env::var(reference).ok())
        .filter(|value| !value.trim().is_empty())
}

fn endpoint(base: &str, suffix: &str) -> String {
    if base.ends_with(suffix) {
        base.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), suffix.trim_start_matches('/'))
    }
}

async fn complete_openai_compatible(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f32,
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }
    messages.push(json!({ "role": "user", "content": user_prompt }));

    let mut request = client
        .post(endpoint(&provider.endpoint_url, "/chat/completions"))
        .json(&json!({
            "model": provider.model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }));

    if let Some(token) = provider_token(provider) {
        request = request.bearer_auth(token);
    }

    let response = request
        .send()
        .await
        .map_err(|cause| format!("provider request failed: {cause}"))?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|cause| format!("provider response parse failed: {cause}"))?;
    if !status.is_success() {
        return Err(format!("provider returned {status}: {payload}"));
    }

    let text = payload
        .pointer("/choices/0/message/content")
        .and_then(value_as_text)
        .or_else(|| payload.pointer("/choices/0/text").and_then(value_as_text))
        .unwrap_or_default();
    let _prompt_tokens = usage_tokens(&payload, "prompt_tokens");
    let _completion_tokens = usage_tokens(&payload, "completion_tokens");

    Ok(CompletionResult {
        text,
    })
}

async fn complete_anthropic(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    let mut request = client
        .post(endpoint(&provider.endpoint_url, "/messages"))
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": provider.model_name,
            "system": system_prompt,
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": user_prompt }],
        }));

    if let Some(token) = provider_token(provider) {
        request = request.header("x-api-key", token);
    }

    let response = request
        .send()
        .await
        .map_err(|cause| format!("provider request failed: {cause}"))?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|cause| format!("provider response parse failed: {cause}"))?;
    if !status.is_success() {
        return Err(format!("provider returned {status}: {payload}"));
    }

    let text = payload
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let _prompt_tokens = usage_tokens(&payload, "input_tokens");
    let _completion_tokens = usage_tokens(&payload, "output_tokens");

    Ok(CompletionResult {
        text,
    })
}

async fn complete_ollama(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<CompletionResult, String> {
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }
    messages.push(json!({ "role": "user", "content": user_prompt }));

    let response = client
        .post(endpoint(&provider.endpoint_url, "/chat"))
        .json(&json!({
            "model": provider.model_name,
            "messages": messages,
            "stream": false,
        }))
        .send()
        .await
        .map_err(|cause| format!("provider request failed: {cause}"))?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|cause| format!("provider response parse failed: {cause}"))?;
    if !status.is_success() {
        return Err(format!("provider returned {status}: {payload}"));
    }

    let text = payload
        .pointer("/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    Ok(CompletionResult {
        text,
    })
}

async fn embed_openai_compatible(
    client: &reqwest::Client,
    provider: &LlmProvider,
    content: &str,
) -> Result<Vec<f32>, String> {
    let mut request = client
        .post(endpoint(&provider.endpoint_url, "/embeddings"))
        .json(&json!({
            "model": provider.model_name,
            "input": content,
        }));

    if let Some(token) = provider_token(provider) {
        request = request.bearer_auth(token);
    }

    let response = request
        .send()
        .await
        .map_err(|cause| format!("embedding request failed: {cause}"))?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|cause| format!("embedding response parse failed: {cause}"))?;
    if !status.is_success() {
        return Err(format!("embedding provider returned {status}: {payload}"));
    }

    parse_embedding(&payload)
}

async fn embed_ollama(
    client: &reqwest::Client,
    provider: &LlmProvider,
    content: &str,
) -> Result<Vec<f32>, String> {
    let response = client
        .post(endpoint(&provider.endpoint_url, "/embeddings"))
        .json(&json!({
            "model": provider.model_name,
            "prompt": content,
        }))
        .send()
        .await
        .map_err(|cause| format!("embedding request failed: {cause}"))?;
    let status = response.status();
    let payload = response
        .json::<Value>()
        .await
        .map_err(|cause| format!("embedding response parse failed: {cause}"))?;
    if !status.is_success() {
        return Err(format!("embedding provider returned {status}: {payload}"));
    }

    payload
        .get("embedding")
        .and_then(Value::as_array)
        .map(|values| value_array_to_f32(values))
        .filter(|embedding| !embedding.is_empty())
        .ok_or_else(|| "embedding payload did not include an embedding vector".to_string())
}

fn parse_embedding(payload: &Value) -> Result<Vec<f32>, String> {
    payload
        .pointer("/data/0/embedding")
        .and_then(Value::as_array)
        .map(|values| value_array_to_f32(values))
        .filter(|embedding| !embedding.is_empty())
        .ok_or_else(|| "embedding payload did not include an embedding vector".to_string())
}

fn value_array_to_f32(values: &[Value]) -> Vec<f32> {
    values
        .iter()
        .filter_map(Value::as_f64)
        .map(|value| value as f32)
        .collect()
}

fn value_as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let collected = parts
                .iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or_else(|| part.get("content").and_then(Value::as_str).map(ToOwned::to_owned))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if collected.is_empty() {
                None
            } else {
                Some(collected)
            }
        }
        _ => None,
    }
}

fn usage_tokens(payload: &Value, key: &str) -> i32 {
    payload
        .get("usage")
        .and_then(|usage| usage.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::{parse_embedding, value_as_text};

    #[test]
    fn parses_embedding_payloads() {
        let embedding = parse_embedding(&json!({
            "data": [{
                "embedding": [0.1, 0.2, 0.3, 0.4]
            }]
        }))
        .unwrap();

        assert_eq!(embedding, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn flattens_structured_text_parts() {
        let text = value_as_text(&json!([
            { "text": "alpha" },
            { "text": "beta" }
        ]))
        .unwrap();

        assert_eq!(text, "alpha\nbeta");
    }
}
