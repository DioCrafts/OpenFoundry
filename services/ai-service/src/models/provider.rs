use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderRoutingRules {
    #[serde(default)]
    pub use_cases: Vec<String>,
    #[serde(default)]
    pub preferred_regions: Vec<String>,
    #[serde(default)]
    pub fallback_provider_ids: Vec<Uuid>,
    #[serde(default = "default_routing_weight")]
    pub weight: i32,
    #[serde(default = "default_context_tokens")]
    pub max_context_tokens: i32,
    #[serde(default = "default_network_scope")]
    pub network_scope: String,
    #[serde(default = "default_supported_modalities")]
    pub supported_modalities: Vec<String>,
    #[serde(default)]
    pub input_cost_per_1k_tokens_usd: f32,
    #[serde(default)]
    pub output_cost_per_1k_tokens_usd: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthState {
    pub status: String,
    pub avg_latency_ms: i32,
    pub error_rate: f32,
    pub last_checked_at: DateTime<Utc>,
}

impl Default for ProviderHealthState {
    fn default() -> Self {
        Self {
            status: "healthy".to_string(),
            avg_latency_ms: 620,
            error_rate: 0.01,
            last_checked_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub api_mode: String,
    pub credential_reference: Option<String>,
    pub credential_configured: bool,
    pub enabled: bool,
    pub load_balance_weight: i32,
    pub max_output_tokens: i32,
    pub cost_tier: String,
    pub tags: Vec<String>,
    pub route_rules: ProviderRoutingRules,
    pub health_state: ProviderHealthState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProvidersResponse {
    pub data: Vec<LlmProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    #[serde(default = "default_model_name")]
    pub model_name: String,
    #[serde(default = "default_endpoint_url")]
    pub endpoint_url: String,
    #[serde(default = "default_api_mode")]
    pub api_mode: String,
    #[serde(default)]
    pub credential_reference: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_provider_weight")]
    pub load_balance_weight: i32,
    #[serde(default = "default_output_tokens")]
    pub max_output_tokens: i32,
    #[serde(default = "default_cost_tier")]
    pub cost_tier: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub route_rules: Option<ProviderRoutingRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub model_name: Option<String>,
    pub endpoint_url: Option<String>,
    pub api_mode: Option<String>,
    pub credential_reference: Option<String>,
    pub enabled: Option<bool>,
    pub load_balance_weight: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub cost_tier: Option<String>,
    pub tags: Option<Vec<String>>,
    pub route_rules: Option<ProviderRoutingRules>,
    pub health_state: Option<ProviderHealthState>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ProviderRow {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    pub model_name: String,
    pub endpoint_url: String,
    pub api_mode: String,
    pub credential_reference: Option<String>,
    pub enabled: bool,
    pub load_balance_weight: i32,
    pub max_output_tokens: i32,
    pub cost_tier: String,
    pub tags: Json<Vec<String>>,
    pub route_rules: Json<ProviderRoutingRules>,
    pub health_state: Json<ProviderHealthState>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProviderRow> for LlmProvider {
    fn from(value: ProviderRow) -> Self {
        let credential_configured = value
            .credential_reference
            .as_ref()
            .map(|reference| !reference.trim().is_empty())
            .unwrap_or(false);

        Self {
            id: value.id,
            name: value.name,
            provider_type: value.provider_type,
            model_name: value.model_name,
            endpoint_url: value.endpoint_url,
            api_mode: value.api_mode,
            credential_reference: value.credential_reference,
            credential_configured,
            enabled: value.enabled,
            load_balance_weight: value.load_balance_weight,
            max_output_tokens: value.max_output_tokens,
            cost_tier: value.cost_tier,
            tags: value.tags.0,
            route_rules: value.route_rules.0,
            health_state: value.health_state.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn default_routing_weight() -> i32 {
    100
}

fn default_context_tokens() -> i32 {
    32_000
}

fn default_provider_type() -> String {
    "openai".to_string()
}

fn default_network_scope() -> String {
    "public".to_string()
}

fn default_supported_modalities() -> Vec<String> {
    vec!["text".to_string()]
}

fn default_model_name() -> String {
    "gpt-4.1-mini".to_string()
}

fn default_endpoint_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_api_mode() -> String {
    "chat_completions".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_provider_weight() -> i32 {
    100
}

fn default_output_tokens() -> i32 {
    2048
}

fn default_cost_tier() -> String {
    "standard".to_string()
}
