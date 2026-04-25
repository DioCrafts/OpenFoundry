use serde_json::{Value, json};

use crate::models::{conversation::ChatAttachment, provider::LlmProvider};

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub text: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

pub async fn complete_text(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    attachments: &[ChatAttachment],
    temperature: f32,
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    match provider.api_mode.as_str() {
        "chat_completions" => {
            complete_openai_compatible(
                client,
                provider,
                system_prompt,
                user_prompt,
                attachments,
                temperature,
                max_tokens,
            )
            .await
        }
        "messages" => {
            complete_anthropic(
                client,
                provider,
                system_prompt,
                user_prompt,
                attachments,
                max_tokens,
            )
            .await
        }
        "chat" => complete_ollama(client, provider, system_prompt, user_prompt, attachments).await,
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
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            suffix.trim_start_matches('/')
        )
    }
}

async fn complete_openai_compatible(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    attachments: &[ChatAttachment],
    temperature: f32,
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }
    messages.push(json!({
        "role": "user",
        "content": build_openai_user_content(user_prompt, attachments)?,
    }));

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
    let prompt_tokens = usage_tokens(&payload, "prompt_tokens");
    let completion_tokens = usage_tokens(&payload, "completion_tokens");

    Ok(CompletionResult {
        text,
        prompt_tokens,
        completion_tokens,
        total_tokens: usage_tokens(&payload, "total_tokens").max(prompt_tokens + completion_tokens),
    })
}

async fn complete_anthropic(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    attachments: &[ChatAttachment],
    max_tokens: i32,
) -> Result<CompletionResult, String> {
    let mut request = client
        .post(endpoint(&provider.endpoint_url, "/messages"))
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": provider.model_name,
            "system": system_prompt,
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": build_anthropic_user_content(user_prompt, attachments) }],
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
    let prompt_tokens = usage_tokens(&payload, "input_tokens");
    let completion_tokens = usage_tokens(&payload, "output_tokens");

    Ok(CompletionResult {
        text,
        prompt_tokens,
        completion_tokens,
        total_tokens: usage_tokens(&payload, "total_tokens").max(prompt_tokens + completion_tokens),
    })
}

async fn complete_ollama(
    client: &reqwest::Client,
    provider: &LlmProvider,
    system_prompt: &str,
    user_prompt: &str,
    attachments: &[ChatAttachment],
) -> Result<CompletionResult, String> {
    let mut messages = Vec::new();
    if !system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }
    let (ollama_prompt, ollama_images) = build_ollama_user_payload(user_prompt, attachments);
    let mut user_message = json!({ "role": "user", "content": ollama_prompt });
    if !ollama_images.is_empty() {
        user_message["images"] = json!(ollama_images);
    }
    messages.push(user_message);

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
    let prompt_tokens = payload
        .get("prompt_eval_count")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;
    let completion_tokens = payload
        .get("eval_count")
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32;

    Ok(CompletionResult {
        text,
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens + completion_tokens,
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

fn build_openai_user_content(
    user_prompt: &str,
    attachments: &[ChatAttachment],
) -> Result<Value, String> {
    if attachments.is_empty() {
        return Ok(json!(user_prompt));
    }

    let mut parts = vec![json!({ "type": "text", "text": user_prompt })];
    for attachment in attachments {
        match attachment.kind.as_str() {
            "text" => {
                if let Some(text) = attachment
                    .text
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    parts.push(json!({ "type": "text", "text": text }));
                }
            }
            "image_url" => {
                let url = attachment
                    .url
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "image_url attachment requires url".to_string())?;
                parts.push(json!({
                    "type": "image_url",
                    "image_url": { "url": url },
                }));
            }
            "image_base64" => {
                let mime_type = attachment
                    .mime_type
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("image/png");
                let base64_data = attachment
                    .base64_data
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "image_base64 attachment requires base64_data".to_string())?;
                parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{mime_type};base64,{base64_data}"),
                    },
                }));
            }
            other => {
                return Err(format!(
                    "unsupported attachment kind '{other}' for openai-compatible chat"
                ));
            }
        }
    }

    Ok(Value::Array(parts))
}

fn build_anthropic_user_content(user_prompt: &str, attachments: &[ChatAttachment]) -> Vec<Value> {
    let mut parts = vec![json!({ "type": "text", "text": user_prompt })];
    for attachment in attachments {
        match attachment.kind.as_str() {
            "text" => {
                if let Some(text) = attachment
                    .text
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    parts.push(json!({ "type": "text", "text": text }));
                }
            }
            "image_base64" => {
                if let Some(base64_data) = attachment
                    .base64_data
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    parts.push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": attachment
                                .mime_type
                                .clone()
                                .unwrap_or_else(|| "image/png".to_string()),
                            "data": base64_data,
                        },
                    }));
                }
            }
            "image_url" => {
                if let Some(url) = attachment
                    .url
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    parts.push(json!({
                        "type": "text",
                        "text": format!("Referenced image URL: {url}"),
                    }));
                }
            }
            _ => {}
        }
    }
    parts
}

fn build_ollama_user_payload(
    user_prompt: &str,
    attachments: &[ChatAttachment],
) -> (String, Vec<String>) {
    let mut prompt = user_prompt.to_string();
    let mut images = Vec::new();

    for attachment in attachments {
        match attachment.kind.as_str() {
            "text" => {
                if let Some(text) = attachment
                    .text
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    prompt.push_str("\n\nAttachment context:\n");
                    prompt.push_str(text);
                }
            }
            "image_base64" => {
                if let Some(base64_data) = attachment
                    .base64_data
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    images.push(base64_data.clone());
                }
            }
            "image_url" => {
                if let Some(url) = attachment
                    .url
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                {
                    prompt.push_str("\n\nReferenced image URL: ");
                    prompt.push_str(url);
                }
            }
            _ => {}
        }
    }

    (prompt, images)
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
                        .or_else(|| {
                            part.get("content")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        })
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

    use crate::models::conversation::ChatAttachment;

    use super::{build_openai_user_content, parse_embedding, value_as_text};

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

    #[test]
    fn builds_openai_multimodal_content() {
        let content = build_openai_user_content(
            "describe the image",
            &[ChatAttachment {
                kind: "image_url".to_string(),
                name: None,
                mime_type: Some("image/png".to_string()),
                url: Some("https://example.com/sample.png".to_string()),
                base64_data: None,
                text: None,
            }],
        )
        .unwrap();

        let parts = content.as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[1]["type"], "image_url");
    }
}
