use std::{
    collections::HashMap,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
};

use reqwest::Client;
use serde_json::{Value, json};
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Clone)]
pub enum EmbeddingBackend {
    DeterministicHash,
    Provider(EmbeddingProvider),
}

#[derive(Debug, Clone)]
pub struct EmbeddingProvider {
    pub reference: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub api_mode: String,
    pub credential_reference: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct EmbeddingProviderRow {
    id: Uuid,
    model_name: String,
    endpoint_url: String,
    api_mode: String,
    credential_reference: Option<String>,
    enabled: bool,
}

static REMOTE_EMBEDDING_CACHE: OnceLock<Mutex<HashMap<String, Vec<f32>>>> = OnceLock::new();

fn remote_embedding_cache() -> &'static Mutex<HashMap<String, Vec<f32>>> {
    REMOTE_EMBEDDING_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn normalized_provider_reference<'a>(requested: Option<&'a str>, configured: &'a str) -> &'a str {
    requested
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(configured)
        .trim()
}

pub async fn resolve_backend(
    state: &AppState,
    requested: Option<&str>,
) -> Result<EmbeddingBackend, sqlx::Error> {
    let provider_reference =
        normalized_provider_reference(requested, &state.search_embedding_provider);

    if provider_reference.is_empty() || provider_reference == "deterministic-hash" {
        return Ok(EmbeddingBackend::DeterministicHash);
    }

    let Some(provider_id) = provider_reference
        .strip_prefix("provider:")
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        tracing::warn!(
            provider_reference,
            "unknown ontology search embedding provider reference, falling back to deterministic hash"
        );
        return Ok(EmbeddingBackend::DeterministicHash);
    };

    let provider = sqlx::query_as::<_, EmbeddingProviderRow>(
        r#"SELECT id, model_name, endpoint_url, api_mode, credential_reference, enabled
           FROM ai_providers
           WHERE id = $1"#,
    )
    .bind(provider_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(provider) = provider else {
        tracing::warn!(
            provider_reference,
            "embedding provider not found for ontology search, falling back to deterministic hash"
        );
        return Ok(EmbeddingBackend::DeterministicHash);
    };

    if !provider.enabled {
        tracing::warn!(
            provider_reference,
            "embedding provider disabled for ontology search, falling back to deterministic hash"
        );
        return Ok(EmbeddingBackend::DeterministicHash);
    }

    Ok(EmbeddingBackend::Provider(EmbeddingProvider {
        reference: format!("provider:{}", provider.id),
        model_name: provider.model_name,
        endpoint_url: provider.endpoint_url,
        api_mode: provider.api_mode,
        credential_reference: provider.credential_reference,
    }))
}

pub fn backend_reference(backend: &EmbeddingBackend) -> &str {
    match backend {
        EmbeddingBackend::DeterministicHash => "deterministic-hash",
        EmbeddingBackend::Provider(provider) => provider.reference.as_str(),
    }
}

pub fn embed_text(content: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; 16];
    let vector_len = vector.len();

    for (index, token) in content
        .to_lowercase()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .enumerate()
    {
        let token_value = token.bytes().fold(0u32, |accumulator, byte| {
            accumulator.wrapping_add(byte as u32)
        });
        vector[index % vector_len] += (token_value % 997) as f32 / 997.0;
    }

    normalize_embedding(vector)
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

pub fn score(query: &str, text: &str) -> f32 {
    let query_embedding = embed_text(query);
    let text_embedding = embed_text(text);
    cosine_similarity(&query_embedding, &text_embedding)
}

pub async fn embed_with_backend(
    state: &AppState,
    backend: &EmbeddingBackend,
    content: &str,
) -> Result<Vec<f32>, String> {
    match backend {
        EmbeddingBackend::DeterministicHash => Ok(embed_text(content)),
        EmbeddingBackend::Provider(provider) => {
            embed_remote_with_cache(&state.http_client, provider, content).await
        }
    }
}

pub async fn score_with_query_embedding(
    state: &AppState,
    backend: &EmbeddingBackend,
    query_embedding: &[f32],
    text: &str,
) -> Result<f32, String> {
    let text_embedding = embed_with_backend(state, backend, text).await?;
    Ok(cosine_similarity(query_embedding, &text_embedding))
}

fn normalize_embedding(mut vector: Vec<f32>) -> Vec<f32> {
    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in &mut vector {
            *value /= magnitude;
        }
    }
    vector
}

fn cache_key(provider_reference: &str, content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{provider_reference}:{}", hasher.finish())
}

async fn embed_remote_with_cache(
    client: &Client,
    provider: &EmbeddingProvider,
    content: &str,
) -> Result<Vec<f32>, String> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let cache_key = cache_key(&provider.reference, content);
    if let Some(cached) = remote_embedding_cache()
        .lock()
        .expect("embedding cache poisoned")
        .get(&cache_key)
        .cloned()
    {
        return Ok(cached);
    }

    let embedding = match provider.api_mode.as_str() {
        "chat_completions" => embed_openai_compatible(client, provider, content).await?,
        "chat" => embed_ollama(client, provider, content).await?,
        mode => {
            return Err(format!(
                "embedding provider api_mode '{mode}' does not support ontology search embeddings"
            ));
        }
    };

    remote_embedding_cache()
        .lock()
        .expect("embedding cache poisoned")
        .insert(cache_key, embedding.clone());
    Ok(embedding)
}

fn provider_token(provider: &EmbeddingProvider) -> Option<String> {
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

async fn embed_openai_compatible(
    client: &Client,
    provider: &EmbeddingProvider,
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
    client: &Client,
    provider: &EmbeddingProvider,
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
    let mut embedding = Vec::with_capacity(values.len());
    for value in values {
        match value.as_f64() {
            Some(number) => embedding.push(number as f32),
            None => return Vec::new(),
        }
    }
    normalize_embedding(embedding)
}

#[cfg(test)]
mod tests {
    use super::{EmbeddingBackend, backend_reference, score};

    #[test]
    fn similar_text_scores_higher_than_unrelated_text() {
        let related = score("payment risk review", "payment risk review workflow");
        let unrelated = score("payment risk review", "mountain weather and sailing");
        assert!(related > unrelated);
    }

    #[test]
    fn deterministic_backend_metadata_is_stable() {
        let backend = EmbeddingBackend::DeterministicHash;
        assert_eq!(backend_reference(&backend), "deterministic-hash");
    }
}
