use anyhow::Result;
use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};

pub async fn serve(host: &str, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/chat/completions", post(openai_chat))
        .route("/v1/messages", post(anthropic_messages))
        .route("/v1/embeddings", post(openai_embeddings))
        .route("/api/chat", post(ollama_chat))
        .route("/api/embeddings", post(ollama_embeddings));

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    println!("mock provider listening on {}:{}", host, port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn openai_chat(Json(payload): Json<Value>) -> impl IntoResponse {
    let prompt = extract_openai_prompt(&payload);
    Json(json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": render_reply(&prompt),
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": estimate_tokens(&prompt),
            "completion_tokens": 48,
            "total_tokens": estimate_tokens(&prompt) + 48
        }
    }))
}

async fn anthropic_messages(Json(payload): Json<Value>) -> impl IntoResponse {
    let prompt = extract_anthropic_prompt(&payload);
    Json(json!({
        "id": "msg_mock",
        "type": "message",
        "content": [{
            "type": "text",
            "text": render_reply(&prompt),
        }],
        "usage": {
            "input_tokens": estimate_tokens(&prompt),
            "output_tokens": 48
        }
    }))
}

async fn ollama_chat(Json(payload): Json<Value>) -> impl IntoResponse {
    let prompt = extract_openai_prompt(&payload);
    Json(json!({
        "message": {
            "role": "assistant",
            "content": render_reply(&prompt)
        },
        "prompt_eval_count": estimate_tokens(&prompt),
        "eval_count": 48
    }))
}

async fn openai_embeddings(Json(payload): Json<Value>) -> impl IntoResponse {
    let input = payload
        .get("input")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Json(json!({
        "object": "list",
        "data": [{
            "index": 0,
            "embedding": embed(input)
        }],
        "usage": {
            "prompt_tokens": estimate_tokens(input),
            "total_tokens": estimate_tokens(input)
        }
    }))
}

async fn ollama_embeddings(Json(payload): Json<Value>) -> impl IntoResponse {
    let prompt = payload
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Json(json!({
        "embedding": embed(prompt)
    }))
}

fn render_reply(prompt: &str) -> String {
    let snippet = prompt
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(prompt)
        .trim();

    format!(
        "Mock provider grounded the answer using the supplied context and tool results. Focus: {}",
        truncate(snippet, 180)
    )
}

fn extract_openai_prompt(payload: &Value) -> String {
    payload
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| {
                    Some(format!(
                        "{}: {}",
                        message.get("role")?.as_str()?,
                        flatten_content(message.get("content")?)
                    ))
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn extract_anthropic_prompt(payload: &Value) -> String {
    let system = payload
        .get("system")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let messages = payload
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| {
                    Some(format!(
                        "{}: {}",
                        message.get("role")?.as_str()?,
                        flatten_content(message.get("content")?)
                    ))
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!("{system}\n{messages}")
}

fn flatten_content(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .or_else(|| item.as_str().map(ToOwned::to_owned))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => value.to_string(),
    }
}

fn embed(content: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; 48];
    let vector_len = vector.len();
    for (index, token) in content
        .to_lowercase()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .enumerate()
    {
        let hash = token
            .bytes()
            .fold(0_u64, |acc, byte| acc.wrapping_mul(33).wrapping_add(byte as u64));
        vector[index % vector_len] += (hash % 997) as f32 / 997.0;
    }

    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in &mut vector {
            *value /= magnitude;
        }
    }

    vector
}

fn estimate_tokens(content: &str) -> i32 {
    ((content.split_whitespace().count() as f32) * 1.35).ceil() as i32
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
