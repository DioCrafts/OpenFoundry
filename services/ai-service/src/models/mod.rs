pub mod agent;
pub mod conversation;
pub mod knowledge_base;
pub mod prompt_template;
pub mod provider;
pub mod tool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPlatformOverview {
    pub provider_count: i64,
    pub private_provider_count: i64,
    pub multimodal_provider_count: i64,
    pub prompt_count: i64,
    pub knowledge_base_count: i64,
    pub indexed_document_count: i64,
    pub indexed_chunk_count: i64,
    pub agent_count: i64,
    pub conversation_count: i64,
    pub cache_entry_count: i64,
    pub cache_hit_rate: f32,
    pub blocked_guardrail_events: i64,
    pub llm_prompt_tokens: i64,
    pub llm_completion_tokens: i64,
    pub estimated_llm_cost_usd: f64,
    pub benchmark_run_count: i64,
}
