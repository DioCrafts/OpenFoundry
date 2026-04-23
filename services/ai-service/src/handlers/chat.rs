use std::collections::BTreeSet;

use axum::{
	extract::{Path, State},
	Json,
};
use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{query_as, query_scalar, types::Json as SqlJson, FromRow};
use uuid::Uuid;

use crate::{
	domain::{copilot, evaluation, llm, rag},
	models::{
		conversation::{
			ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Conversation,
			ConversationRow, ConversationSummary, CopilotRequest, CopilotResponse,
			EvaluateGuardrailsRequest, EvaluateGuardrailsResponse, GuardrailVerdict,
			ListConversationsResponse, SemanticCacheMetadata,
		},
		knowledge_base::{KnowledgeDocument, KnowledgeDocumentRow, KnowledgeSearchResult},
		prompt_template::{PromptTemplate, PromptTemplateRow},
		provider::{
			CreateProviderRequest, ListProvidersResponse, LlmProvider, ProviderHealthState,
			ProviderRow, UpdateProviderRequest,
		},
		AiPlatformOverview,
	},
	AppState,
};

use super::{bad_request, db_error, not_found, ServiceResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedChatPayload {
	reply: String,
	citations: Vec<KnowledgeSearchResult>,
	completion_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedCopilotPayload {
	answer: String,
	suggested_sql: Option<String>,
	pipeline_suggestions: Vec<String>,
	ontology_hints: Vec<String>,
	cited_knowledge: Vec<KnowledgeSearchResult>,
}

#[derive(Debug, FromRow)]
struct CacheRow {
	id: Uuid,
	cache_key: String,
	fingerprint: SqlJson<Vec<f32>>,
	response: SqlJson<Value>,
	provider_id: Option<Uuid>,
}

async fn load_provider_row(
	db: &sqlx::PgPool,
	provider_id: Uuid,
) -> Result<Option<ProviderRow>, sqlx::Error> {
	query_as::<_, ProviderRow>(
		r#"
		SELECT
			id,
			name,
			provider_type,
			model_name,
			endpoint_url,
			api_mode,
			credential_reference,
			enabled,
			load_balance_weight,
			max_output_tokens,
			cost_tier,
			tags,
			route_rules,
			health_state,
			created_at,
			updated_at
		FROM ai_providers
		WHERE id = $1
		"#,
	)
	.bind(provider_id)
	.fetch_optional(db)
	.await
}

async fn load_provider_rows(db: &sqlx::PgPool) -> Result<Vec<ProviderRow>, sqlx::Error> {
	query_as::<_, ProviderRow>(
		r#"
		SELECT
			id,
			name,
			provider_type,
			model_name,
			endpoint_url,
			api_mode,
			credential_reference,
			enabled,
			load_balance_weight,
			max_output_tokens,
			cost_tier,
			tags,
			route_rules,
			health_state,
			created_at,
			updated_at
		FROM ai_providers
		ORDER BY updated_at DESC, created_at DESC
		"#,
	)
	.fetch_all(db)
	.await
}

async fn load_prompt_row(
	db: &sqlx::PgPool,
	prompt_id: Uuid,
) -> Result<Option<PromptTemplateRow>, sqlx::Error> {
	query_as::<_, PromptTemplateRow>(
		r#"
		SELECT
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions,
			created_at,
			updated_at
		FROM ai_prompt_templates
		WHERE id = $1
		"#,
	)
	.bind(prompt_id)
	.fetch_optional(db)
	.await
}

async fn load_conversation_row(
	db: &sqlx::PgPool,
	conversation_id: Uuid,
) -> Result<Option<ConversationRow>, sqlx::Error> {
	query_as::<_, ConversationRow>(
		r#"
		SELECT
			id,
			title,
			messages,
			provider_id,
			last_cache_hit,
			last_guardrail_blocked,
			created_at,
			last_activity_at
		FROM ai_conversations
		WHERE id = $1
		"#,
	)
	.bind(conversation_id)
	.fetch_optional(db)
	.await
}

async fn load_documents_for_bases(
	db: &sqlx::PgPool,
	knowledge_base_ids: &[Uuid],
) -> Result<Vec<KnowledgeDocument>, sqlx::Error> {
	let mut documents = Vec::new();
	for knowledge_base_id in knowledge_base_ids {
		let rows = query_as::<_, KnowledgeDocumentRow>(
			r#"
			SELECT
				id,
				knowledge_base_id,
				title,
				content,
				source_uri,
				metadata,
				status,
				chunk_count,
				chunks,
				created_at,
				updated_at
			FROM ai_knowledge_documents
			WHERE knowledge_base_id = $1
			ORDER BY updated_at DESC
			"#,
		)
		.bind(*knowledge_base_id)
		.fetch_all(db)
		.await?;

		documents.extend(rows.into_iter().map(Into::into));
	}

	Ok(documents)
}

async fn find_cached_response<T>(
	db: &sqlx::PgPool,
	kind: &str,
	prompt: &str,
) -> Result<Option<(T, SemanticCacheMetadata, Option<Uuid>)>, sqlx::Error>
where
	T: DeserializeOwned,
{
	let rows = query_as::<_, CacheRow>(
		r#"
		SELECT
			id,
			cache_key,
			fingerprint,
			response,
			provider_id
		FROM ai_semantic_cache
		WHERE kind = $1
		ORDER BY last_hit_at DESC
		LIMIT 64
		"#,
	)
	.bind(kind)
	.fetch_all(db)
	.await?;

	let exact_key = llm::cache::cache_key(kind, prompt);
	let fingerprint = llm::cache::fingerprint(prompt);
	let mut best_match: Option<(CacheRow, f32)> = None;

	for row in rows {
		let score = if row.cache_key == exact_key {
			1.0
		} else {
			llm::cache::cosine_similarity(&fingerprint, &row.fingerprint.0)
		};

		if score < 0.92 {
			continue;
		}

		if best_match
			.as_ref()
			.map(|(_, current_score)| score > *current_score)
			.unwrap_or(true)
		{
			best_match = Some((row, score));
		}
	}

	let Some((row, score)) = best_match else {
		return Ok(None);
	};

	sqlx::query(
		"UPDATE ai_semantic_cache SET hit_count = hit_count + 1, last_hit_at = NOW() WHERE id = $1",
	)
	.bind(row.id)
	.execute(db)
	.await?;

	let payload = serde_json::from_value(row.response.0).ok();
	Ok(payload.map(|payload| {
		(
			payload,
			SemanticCacheMetadata {
				cache_key: row.cache_key,
				hit: true,
				similarity_score: score,
			},
			row.provider_id,
		)
	}))
}

async fn upsert_cached_response<T>(
	db: &sqlx::PgPool,
	kind: &str,
	prompt: &str,
	provider_id: Option<Uuid>,
	payload: &T,
) -> Result<SemanticCacheMetadata, sqlx::Error>
where
	T: Serialize,
{
	let cache_key = llm::cache::cache_key(kind, prompt);
	let normalized_prompt = llm::cache::normalize_text(prompt);
	let fingerprint = llm::cache::fingerprint(prompt);
	let response = serde_json::to_value(payload).unwrap_or_else(|_| json!(null));

	sqlx::query(
		r#"
		INSERT INTO ai_semantic_cache (
			id,
			kind,
			cache_key,
			normalized_prompt,
			fingerprint,
			response,
			provider_id,
			hit_count,
			last_hit_at
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, 0, NOW())
		ON CONFLICT (kind, cache_key)
		DO UPDATE SET
			normalized_prompt = EXCLUDED.normalized_prompt,
			fingerprint = EXCLUDED.fingerprint,
			response = EXCLUDED.response,
			provider_id = EXCLUDED.provider_id,
			last_hit_at = NOW()
		"#,
	)
	.bind(Uuid::now_v7())
	.bind(kind)
	.bind(&cache_key)
	.bind(normalized_prompt)
	.bind(SqlJson(fingerprint))
	.bind(SqlJson(response))
	.bind(provider_id)
	.execute(db)
	.await?;

	Ok(SemanticCacheMetadata {
		cache_key,
		hit: false,
		similarity_score: 0.0,
	})
}

async fn persist_conversation(
	db: &sqlx::PgPool,
	conversation_id: Option<Uuid>,
	user_message: &str,
	reply: &str,
	provider_id: Uuid,
	citations: &[KnowledgeSearchResult],
	guardrail: &GuardrailVerdict,
	cache_hit: bool,
) -> Result<Uuid, sqlx::Error> {
	let now = Utc::now();
	let user_entry = ChatMessage {
		role: "user".to_string(),
		content: user_message.to_string(),
		provider_id: None,
		tool_name: None,
		citations: Vec::new(),
		guardrail_verdict: Some(guardrail.clone()),
		created_at: now,
	};
	let assistant_entry = ChatMessage {
		role: "assistant".to_string(),
		content: reply.to_string(),
		provider_id: Some(provider_id),
		tool_name: None,
		citations: citations.to_vec(),
		guardrail_verdict: None,
		created_at: now,
	};

	if let Some(conversation_id) = conversation_id {
		if let Some(current) = load_conversation_row(db, conversation_id).await? {
			let mut messages = current.messages.0;
			messages.push(user_entry);
			messages.push(assistant_entry);

			sqlx::query(
				"UPDATE ai_conversations SET messages = $2, provider_id = $3, last_cache_hit = $4, last_guardrail_blocked = $5, last_activity_at = NOW() WHERE id = $1",
			)
			.bind(conversation_id)
			.bind(SqlJson(messages))
			.bind(provider_id)
			.bind(cache_hit)
			.bind(guardrail.blocked)
			.execute(db)
			.await?;

			return Ok(conversation_id);
		}
	}

	let new_id = Uuid::now_v7();
	let title = summarize_title(user_message);
	sqlx::query(
		"INSERT INTO ai_conversations (id, title, messages, provider_id, last_cache_hit, last_guardrail_blocked) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(new_id)
	.bind(title)
	.bind(SqlJson(vec![user_entry, assistant_entry]))
	.bind(provider_id)
	.bind(cache_hit)
	.bind(guardrail.blocked)
	.execute(db)
	.await?;

	Ok(new_id)
}

fn summarize_title(content: &str) -> String {
	let mut chars = content.trim().chars();
	let title = chars.by_ref().take(60).collect::<String>();
	if chars.next().is_some() {
		format!("{title}...")
	} else if title.is_empty() {
		"New conversation".to_string()
	} else {
		title
	}
}

fn conversation_summary(conversation: Conversation) -> ConversationSummary {
	let last_message_preview = conversation
		.messages
		.last()
		.map(|message| summarize_title(&message.content))
		.unwrap_or_else(|| "No messages yet".to_string());

	ConversationSummary {
		id: conversation.id,
		title: conversation.title,
		last_message_preview,
		provider_id: conversation.provider_id,
		message_count: conversation.messages.len() as i32,
		last_cache_hit: conversation.last_cache_hit,
		last_activity_at: conversation.last_activity_at,
	}
}

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<AiPlatformOverview> {
	let provider_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_providers")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let prompt_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_prompt_templates WHERE status = 'active'")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let knowledge_base_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_knowledge_bases")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let indexed_document_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_knowledge_documents")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let indexed_chunk_count = query_scalar::<_, i64>("SELECT COALESCE(SUM(chunk_count), 0) FROM ai_knowledge_documents")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let agent_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_agents")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let conversation_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_conversations")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let cache_entry_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ai_semantic_cache")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let total_cache_hits = query_scalar::<_, i64>("SELECT COALESCE(SUM(hit_count), 0) FROM ai_semantic_cache")
		.fetch_one(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	let blocked_guardrail_events = query_scalar::<_, i64>(
		"SELECT COUNT(*) FROM ai_conversations WHERE last_guardrail_blocked = TRUE",
	)
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(AiPlatformOverview {
		provider_count,
		prompt_count,
		knowledge_base_count,
		indexed_document_count,
		indexed_chunk_count,
		agent_count,
		conversation_count,
		cache_entry_count,
		cache_hit_rate: evaluation::cache_hit_rate(cache_entry_count, total_cache_hits),
		blocked_guardrail_events,
	}))
}

pub async fn list_providers(State(state): State<AppState>) -> ServiceResult<ListProvidersResponse> {
	let rows = load_provider_rows(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
	Ok(Json(ListProvidersResponse {
		data: rows.into_iter().map(Into::into).collect(),
	}))
}

pub async fn create_provider(
	State(state): State<AppState>,
	Json(body): Json<CreateProviderRequest>,
) -> ServiceResult<LlmProvider> {
	if body.name.trim().is_empty() {
		return Err(bad_request("provider name is required"));
	}

	let route_rules = body.route_rules.unwrap_or_default();
	let health_state = ProviderHealthState {
		status: if body.enabled {
			"healthy".to_string()
		} else {
			"offline".to_string()
		},
		..ProviderHealthState::default()
	};

	let row = query_as::<_, ProviderRow>(
		r#"
		INSERT INTO ai_providers (
			id,
			name,
			provider_type,
			model_name,
			endpoint_url,
			api_mode,
			credential_reference,
			enabled,
			load_balance_weight,
			max_output_tokens,
			cost_tier,
			tags,
			route_rules,
			health_state
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
		RETURNING
			id,
			name,
			provider_type,
			model_name,
			endpoint_url,
			api_mode,
			credential_reference,
			enabled,
			load_balance_weight,
			max_output_tokens,
			cost_tier,
			tags,
			route_rules,
			health_state,
			created_at,
			updated_at
		"#,
	)
	.bind(Uuid::now_v7())
	.bind(body.name.trim())
	.bind(body.provider_type)
	.bind(body.model_name)
	.bind(body.endpoint_url)
	.bind(body.api_mode)
	.bind(body.credential_reference)
	.bind(body.enabled)
	.bind(body.load_balance_weight)
	.bind(body.max_output_tokens)
	.bind(body.cost_tier)
	.bind(SqlJson(body.tags))
	.bind(SqlJson(route_rules))
	.bind(SqlJson(health_state))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn update_provider(
	State(state): State<AppState>,
	Path(provider_id): Path<Uuid>,
	Json(body): Json<UpdateProviderRequest>,
) -> ServiceResult<LlmProvider> {
	let Some(current) = load_provider_row(&state.db, provider_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("provider not found"));
	};

	let provider: LlmProvider = current.into();
	let mut health_state = body.health_state.unwrap_or(provider.health_state);
	if let Some(enabled) = body.enabled {
		if !enabled {
			health_state.status = "offline".to_string();
		} else if health_state.status == "offline" {
			health_state.status = "healthy".to_string();
		}
	}

	let row = query_as::<_, ProviderRow>(
		r#"
		UPDATE ai_providers
		SET name = $2,
			provider_type = $3,
			model_name = $4,
			endpoint_url = $5,
			api_mode = $6,
			credential_reference = $7,
			enabled = $8,
			load_balance_weight = $9,
			max_output_tokens = $10,
			cost_tier = $11,
			tags = $12,
			route_rules = $13,
			health_state = $14,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			provider_type,
			model_name,
			endpoint_url,
			api_mode,
			credential_reference,
			enabled,
			load_balance_weight,
			max_output_tokens,
			cost_tier,
			tags,
			route_rules,
			health_state,
			created_at,
			updated_at
		"#,
	)
	.bind(provider_id)
	.bind(body.name.unwrap_or(provider.name))
	.bind(body.provider_type.unwrap_or(provider.provider_type))
	.bind(body.model_name.unwrap_or(provider.model_name))
	.bind(body.endpoint_url.unwrap_or(provider.endpoint_url))
	.bind(body.api_mode.unwrap_or(provider.api_mode))
	.bind(body.credential_reference.or(provider.credential_reference))
	.bind(body.enabled.unwrap_or(provider.enabled))
	.bind(body.load_balance_weight.unwrap_or(provider.load_balance_weight))
	.bind(body.max_output_tokens.unwrap_or(provider.max_output_tokens))
	.bind(body.cost_tier.unwrap_or(provider.cost_tier))
	.bind(SqlJson(body.tags.unwrap_or(provider.tags)))
	.bind(SqlJson(body.route_rules.unwrap_or(provider.route_rules)))
	.bind(SqlJson(health_state))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn list_conversations(
	State(state): State<AppState>,
) -> ServiceResult<ListConversationsResponse> {
	let rows = query_as::<_, ConversationRow>(
		r#"
		SELECT
			id,
			title,
			messages,
			provider_id,
			last_cache_hit,
			last_guardrail_blocked,
			created_at,
			last_activity_at
		FROM ai_conversations
		ORDER BY last_activity_at DESC, created_at DESC
		"#,
	)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(ListConversationsResponse {
		data: rows
			.into_iter()
			.map(Into::<Conversation>::into)
			.map(conversation_summary)
			.collect(),
	}))
}

pub async fn get_conversation(
	State(state): State<AppState>,
	Path(conversation_id): Path<Uuid>,
) -> ServiceResult<Conversation> {
	let Some(row) = load_conversation_row(&state.db, conversation_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("conversation not found"));
	};

	Ok(Json(row.into()))
}

pub async fn create_chat_completion(
	State(state): State<AppState>,
	Json(body): Json<ChatCompletionRequest>,
) -> ServiceResult<ChatCompletionResponse> {
	if body.user_message.trim().is_empty() {
		return Err(bad_request("chat completion requires a user message"));
	}

	let providers = load_provider_rows(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?
		.into_iter()
		.map(Into::into)
		.collect::<Vec<LlmProvider>>();
	if providers.is_empty() {
		return Err(not_found("no AI providers configured"));
	}

	let base_prompt = if let Some(prompt_template_id) = body.prompt_template_id {
		let Some(prompt_row) = load_prompt_row(&state.db, prompt_template_id)
			.await
			.map_err(|cause| db_error(&cause))?
		else {
			return Err(not_found("prompt template not found"));
		};
		let prompt: PromptTemplate = prompt_row.into();
		let (rendered, _) = llm::provider::interpolate_template(
			&prompt.current_version.content,
			&body.prompt_variables,
			false,
		);
		if let Some(system_prompt) = body.system_prompt {
			format!("{system_prompt}\n\n{rendered}")
		} else {
			rendered
		}
	} else {
		body.system_prompt.unwrap_or_else(|| {
			"You are the OpenFoundry platform copilot. Provide grounded operational guidance.".to_string()
		})
	};

	let guardrail = llm::guardrails::evaluate_text(&body.user_message);
	let knowledge_hits = if let Some(knowledge_base_id) = body.knowledge_base_id {
		let documents = load_documents_for_bases(&state.db, &[knowledge_base_id])
			.await
			.map_err(|cause| db_error(&cause))?;
		rag::retriever::search(&body.user_message, &documents, 4, 0.55)
	} else {
		Vec::new()
	};

	let prompt_used = format!(
		"{base_prompt}\n\nUser request: {}\n\nRetrieved context count: {}",
		guardrail.redacted_text,
		knowledge_hits.len()
	);

	let routed = llm::gateway::route_providers(&providers, body.preferred_provider_id, "chat");
	let provider = llm::gateway::select_provider(&routed, body.fallback_enabled)
		.ok_or_else(|| not_found("no AI provider available"))?;

	if let Some((payload, cache, cached_provider_id)) = find_cached_response::<CachedChatPayload>(
		&state.db,
		"chat",
		&prompt_used,
	)
	.await
	.map_err(|cause| db_error(&cause))?
	{
		let provider_id = cached_provider_id.unwrap_or(provider.id);
		let conversation_id = persist_conversation(
			&state.db,
			body.conversation_id,
			&body.user_message,
			&payload.reply,
			provider_id,
			&payload.citations,
			&guardrail,
			true,
		)
		.await
		.map_err(|cause| db_error(&cause))?;

		let provider_name = providers
			.iter()
			.find(|candidate| candidate.id == provider_id)
			.map(|candidate| candidate.name.clone())
			.unwrap_or_else(|| provider.name.clone());

		return Ok(Json(ChatCompletionResponse {
			conversation_id,
			provider_id,
			provider_name,
			reply: payload.reply,
			citations: payload.citations,
			guardrail,
			cache,
			prompt_used,
			completion_tokens: payload.completion_tokens,
			created_at: Utc::now(),
		}));
	}

	let reply = if guardrail.blocked {
		"Guardrails blocked this request. Remove prompt-injection or toxic content and retry.".to_string()
	} else {
		llm::gateway::synthesize_completion(&provider, &prompt_used, &knowledge_hits)
	};
	let completion_tokens = llm::gateway::estimate_tokens(&reply).min(body.max_tokens);
	let payload = CachedChatPayload {
		reply: reply.clone(),
		citations: knowledge_hits.clone(),
		completion_tokens,
	};
	let cache = upsert_cached_response(&state.db, "chat", &prompt_used, Some(provider.id), &payload)
		.await
		.map_err(|cause| db_error(&cause))?;
	let conversation_id = persist_conversation(
		&state.db,
		body.conversation_id,
		&body.user_message,
		&reply,
		provider.id,
		&knowledge_hits,
		&guardrail,
		false,
	)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(ChatCompletionResponse {
		conversation_id,
		provider_id: provider.id,
		provider_name: provider.name,
		reply,
		citations: knowledge_hits,
		guardrail,
		cache,
		prompt_used,
		completion_tokens,
		created_at: Utc::now(),
	}))
}

pub async fn ask_copilot(
	State(state): State<AppState>,
	Json(body): Json<CopilotRequest>,
) -> ServiceResult<CopilotResponse> {
	if body.question.trim().is_empty() {
		return Err(bad_request("copilot question is required"));
	}

	let providers = load_provider_rows(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?
		.into_iter()
		.map(Into::into)
		.collect::<Vec<LlmProvider>>();
	if providers.is_empty() {
		return Err(not_found("no AI providers configured"));
	}

	let prompt_used = format!(
		"question={} datasets={:?} ontology={:?} knowledge_bases={:?}",
		body.question,
		body.dataset_ids,
		body.ontology_type_ids,
		body.knowledge_base_ids
	);

	if let Some((payload, cache, cached_provider_id)) = find_cached_response::<CachedCopilotPayload>(
		&state.db,
		"copilot",
		&prompt_used,
	)
	.await
	.map_err(|cause| db_error(&cause))?
	{
		let provider_name = cached_provider_id
			.and_then(|provider_id| {
				providers
					.iter()
					.find(|candidate| candidate.id == provider_id)
					.map(|candidate| candidate.name.clone())
			})
			.unwrap_or_else(|| providers[0].name.clone());

		return Ok(Json(CopilotResponse {
			answer: payload.answer,
			suggested_sql: payload.suggested_sql,
			pipeline_suggestions: payload.pipeline_suggestions,
			ontology_hints: payload.ontology_hints,
			cited_knowledge: payload.cited_knowledge,
			provider_name,
			cache,
			created_at: Utc::now(),
		}));
	}

	let provider = llm::gateway::select_provider(
		&llm::gateway::route_providers(&providers, body.preferred_provider_id, "copilot"),
		true,
	)
	.ok_or_else(|| not_found("no AI provider available"))?;

	let documents = load_documents_for_bases(&state.db, &body.knowledge_base_ids)
		.await
		.map_err(|cause| db_error(&cause))?;
	let cited_knowledge = rag::retriever::search(&body.question, &documents, 6, 0.55);
	let guardrail = llm::guardrails::evaluate_text(&body.question);

	let draft = copilot::assist(
		&body.question,
		&body.dataset_ids,
		&body.ontology_type_ids,
		&cited_knowledge,
		body.include_sql,
		body.include_pipeline_plan,
	);

	let payload = CachedCopilotPayload {
		answer: if guardrail.blocked {
			"Guardrails blocked this copilot request. Remove unsafe instructions and retry.".to_string()
		} else {
			draft.answer.clone()
		},
		suggested_sql: if guardrail.blocked { None } else { draft.suggested_sql.clone() },
		pipeline_suggestions: if guardrail.blocked {
			Vec::new()
		} else {
			draft.pipeline_suggestions.clone()
		},
		ontology_hints: if guardrail.blocked {
			Vec::new()
		} else {
			draft.ontology_hints.clone()
		},
		cited_knowledge: cited_knowledge.clone(),
	};

	let cache = upsert_cached_response(&state.db, "copilot", &prompt_used, Some(provider.id), &payload)
		.await
		.map_err(|cause| db_error(&cause))?;

	Ok(Json(CopilotResponse {
		answer: payload.answer,
		suggested_sql: payload.suggested_sql,
		pipeline_suggestions: payload.pipeline_suggestions,
		ontology_hints: payload.ontology_hints,
		cited_knowledge: payload.cited_knowledge,
		provider_name: provider.name,
		cache,
		created_at: Utc::now(),
	}))
}

pub async fn evaluate_guardrails(
	State(_state): State<AppState>,
	Json(body): Json<EvaluateGuardrailsRequest>,
) -> ServiceResult<EvaluateGuardrailsResponse> {
	if body.content.trim().is_empty() {
		return Err(bad_request("guardrail evaluation requires content"));
	}

	let verdict = llm::guardrails::evaluate_text(&body.content);
	let mut recommendations = BTreeSet::new();

	if verdict.blocked {
		recommendations.insert("Remove prompt-injection or toxic content before retrying.".to_string());
	}
	for flag in &verdict.flags {
		if flag.kind.starts_with("pii_") {
			recommendations.insert("Redact PII before routing prompts to external LLM providers.".to_string());
		}
	}
	if recommendations.is_empty() {
		recommendations.insert("No blocking issues detected; response is safe to continue.".to_string());
	}

	Ok(Json(EvaluateGuardrailsResponse {
		verdict: verdict.clone(),
		risk_score: evaluation::risk_score(&verdict),
		recommendations: recommendations.into_iter().collect(),
	}))
}
