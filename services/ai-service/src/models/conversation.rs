use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow};
use uuid::Uuid;

use crate::models::knowledge_base::KnowledgeSearchResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailFlag {
	pub kind: String,
	pub severity: String,
	pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailVerdict {
	pub status: String,
	pub redacted_text: String,
	pub blocked: bool,
	pub flags: Vec<GuardrailFlag>,
}

impl Default for GuardrailVerdict {
	fn default() -> Self {
		Self {
			status: "passed".to_string(),
			redacted_text: String::new(),
			blocked: false,
			flags: Vec::new(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	pub role: String,
	pub content: String,
	pub provider_id: Option<Uuid>,
	pub tool_name: Option<String>,
	#[serde(default)]
	pub citations: Vec<KnowledgeSearchResult>,
	pub guardrail_verdict: Option<GuardrailVerdict>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCacheMetadata {
	pub cache_key: String,
	pub hit: bool,
	pub similarity_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
	pub id: Uuid,
	pub title: String,
	pub messages: Vec<ChatMessage>,
	pub provider_id: Option<Uuid>,
	pub last_cache_hit: bool,
	pub last_guardrail_blocked: bool,
	pub created_at: DateTime<Utc>,
	pub last_activity_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
	pub id: Uuid,
	pub title: String,
	pub last_message_preview: String,
	pub provider_id: Option<Uuid>,
	pub message_count: i32,
	pub last_cache_hit: bool,
	pub last_activity_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConversationsResponse {
	pub data: Vec<ConversationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
	pub conversation_id: Option<Uuid>,
	pub system_prompt: Option<String>,
	pub user_message: String,
	pub prompt_template_id: Option<Uuid>,
	#[serde(default)]
	pub prompt_variables: Value,
	pub knowledge_base_id: Option<Uuid>,
	pub preferred_provider_id: Option<Uuid>,
	#[serde(default = "default_fallback_enabled")]
	pub fallback_enabled: bool,
	#[serde(default = "default_temperature")]
	pub temperature: f32,
	#[serde(default = "default_max_tokens")]
	pub max_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
	pub conversation_id: Uuid,
	pub provider_id: Uuid,
	pub provider_name: String,
	pub reply: String,
	pub citations: Vec<KnowledgeSearchResult>,
	pub guardrail: GuardrailVerdict,
	pub cache: SemanticCacheMetadata,
	pub prompt_used: String,
	pub completion_tokens: i32,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotRequest {
	pub question: String,
	#[serde(default)]
	pub dataset_ids: Vec<Uuid>,
	#[serde(default)]
	pub ontology_type_ids: Vec<Uuid>,
	#[serde(default)]
	pub knowledge_base_ids: Vec<Uuid>,
	#[serde(default = "default_true")]
	pub include_sql: bool,
	#[serde(default = "default_true")]
	pub include_pipeline_plan: bool,
	pub preferred_provider_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotResponse {
	pub answer: String,
	pub suggested_sql: Option<String>,
	pub pipeline_suggestions: Vec<String>,
	pub ontology_hints: Vec<String>,
	pub cited_knowledge: Vec<KnowledgeSearchResult>,
	pub provider_name: String,
	pub cache: SemanticCacheMetadata,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateGuardrailsRequest {
	pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateGuardrailsResponse {
	pub verdict: GuardrailVerdict,
	pub risk_score: f32,
	pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ConversationRow {
	pub id: Uuid,
	pub title: String,
	pub messages: Json<Vec<ChatMessage>>,
	pub provider_id: Option<Uuid>,
	pub last_cache_hit: bool,
	pub last_guardrail_blocked: bool,
	pub created_at: DateTime<Utc>,
	pub last_activity_at: DateTime<Utc>,
}

impl From<ConversationRow> for Conversation {
	fn from(value: ConversationRow) -> Self {
		Self {
			id: value.id,
			title: value.title,
			messages: value.messages.0,
			provider_id: value.provider_id,
			last_cache_hit: value.last_cache_hit,
			last_guardrail_blocked: value.last_guardrail_blocked,
			created_at: value.created_at,
			last_activity_at: value.last_activity_at,
		}
	}
}

fn default_fallback_enabled() -> bool {
	true
}

fn default_temperature() -> f32 {
	0.2
}

fn default_max_tokens() -> i32 {
	1024
}

fn default_true() -> bool {
	true
}
