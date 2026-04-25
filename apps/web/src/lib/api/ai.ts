import api from './client';

export interface ListResponse<T> {
	data: T[];
}

export interface ProviderRoutingRules {
	use_cases: string[];
	preferred_regions: string[];
	fallback_provider_ids: string[];
	weight: number;
	max_context_tokens: number;
	network_scope: string;
	supported_modalities: string[];
	input_cost_per_1k_tokens_usd: number;
	output_cost_per_1k_tokens_usd: number;
}

export interface ProviderHealthState {
	status: string;
	avg_latency_ms: number;
	error_rate: number;
	last_checked_at: string;
}

export interface LlmProvider {
	id: string;
	name: string;
	provider_type: string;
	model_name: string;
	endpoint_url: string;
	api_mode: string;
	credential_reference: string | null;
	enabled: boolean;
	load_balance_weight: number;
	max_output_tokens: number;
	cost_tier: string;
	tags: string[];
	route_rules: ProviderRoutingRules;
	health_state: ProviderHealthState;
	created_at: string;
	updated_at: string;
}

export interface PromptVersion {
	version_number: number;
	content: string;
	input_variables: string[];
	notes: string;
	created_at: string;
	created_by: string | null;
}

export interface PromptTemplate {
	id: string;
	name: string;
	description: string;
	category: string;
	status: string;
	tags: string[];
	current_version: PromptVersion;
	versions: PromptVersion[];
	created_at: string;
	updated_at: string;
}

export interface RenderPromptResponse {
	prompt_id: string;
	version_number: number;
	rendered_content: string;
	missing_variables: string[];
}

export interface KnowledgeChunk {
	id: string;
	position: number;
	text: string;
	token_count: number;
	embedding: number[];
	metadata: Record<string, unknown>;
}

export interface KnowledgeDocument {
	id: string;
	knowledge_base_id: string;
	title: string;
	content: string;
	source_uri: string | null;
	metadata: Record<string, unknown>;
	status: string;
	chunk_count: number;
	chunks: KnowledgeChunk[];
	created_at: string;
	updated_at: string;
}

export interface KnowledgeBase {
	id: string;
	name: string;
	description: string;
	status: string;
	embedding_provider: string;
	chunking_strategy: string;
	tags: string[];
	document_count: number;
	chunk_count: number;
	created_at: string;
	updated_at: string;
}

export interface KnowledgeSearchResult {
	knowledge_base_id: string;
	document_id: string;
	document_title: string;
	chunk_id: string;
	score: number;
	excerpt: string;
	source_uri: string | null;
	metadata: Record<string, unknown>;
}

export interface SearchKnowledgeBaseResponse {
	knowledge_base_id: string;
	query: string;
	results: KnowledgeSearchResult[];
	retrieved_at: string;
}

export interface ToolDefinition {
	id: string;
	name: string;
	description: string;
	category: string;
	execution_mode: string;
	execution_config: Record<string, unknown>;
	status: string;
	input_schema: Record<string, unknown>;
	output_schema: Record<string, unknown>;
	tags: string[];
	created_at: string;
	updated_at: string;
}

export interface AgentMemorySnapshot {
	short_term_notes: string[];
	long_term_references: string[];
	last_run_summary: string;
}

export interface AgentPlanStep {
	id: string;
	title: string;
	description: string;
	tool_name: string | null;
	status: string;
}

export interface AgentExecutionTrace {
	step_id: string;
	title: string;
	tool_name: string | null;
	observation: string;
	output: Record<string, unknown>;
}

export interface AgentDefinition {
	id: string;
	name: string;
	description: string;
	status: string;
	system_prompt: string;
	objective: string;
	tool_ids: string[];
	planning_strategy: string;
	max_iterations: number;
	memory: AgentMemorySnapshot;
	last_execution_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface AgentExecutionResponse {
	agent_id: string;
	steps: AgentPlanStep[];
	traces: AgentExecutionTrace[];
	final_response: string;
	used_tool_names: string[];
	executed_at: string;
}

export interface GuardrailFlag {
	kind: string;
	description: string;
	severity: string;
	span: string | null;
}

export interface GuardrailVerdict {
	status: string;
	redacted_text: string;
	blocked: boolean;
	flags: GuardrailFlag[];
}

export interface SemanticCacheMetadata {
	cache_key: string;
	hit: boolean;
	similarity_score: number;
}

export interface ChatMessage {
	role: string;
	content: string;
	provider_id: string | null;
	tool_name: string | null;
	citations: KnowledgeSearchResult[];
	attachments: ChatAttachment[];
	guardrail_verdict: GuardrailVerdict | null;
	created_at: string;
}

export interface ChatAttachment {
	kind: string;
	name: string | null;
	mime_type: string | null;
	url: string | null;
	base64_data: string | null;
	text: string | null;
}

export interface LlmUsageSummary {
	prompt_tokens: number;
	completion_tokens: number;
	total_tokens: number;
	estimated_cost_usd: number;
	latency_ms: number;
	network_scope: string;
	cache_hit: boolean;
}

export interface ChatRoutingMetadata {
	requested_private_network: boolean;
	used_private_network: boolean;
	privacy_reason: string | null;
	candidate_provider_ids: string[];
	required_modalities: string[];
}

export interface Conversation {
	id: string;
	title: string;
	messages: ChatMessage[];
	provider_id: string | null;
	last_cache_hit: boolean;
	last_guardrail_blocked: boolean;
	created_at: string;
	last_activity_at: string;
}

export interface ConversationSummary {
	id: string;
	title: string;
	last_message_preview: string;
	provider_id: string | null;
	message_count: number;
	last_cache_hit: boolean;
	last_activity_at: string;
}

export interface ChatCompletionResponse {
	conversation_id: string;
	provider_id: string;
	provider_name: string;
	reply: string;
	citations: KnowledgeSearchResult[];
	guardrail: GuardrailVerdict;
	cache: SemanticCacheMetadata;
	prompt_used: string;
	completion_tokens: number;
	usage: LlmUsageSummary;
	routing: ChatRoutingMetadata;
	created_at: string;
}

export interface CopilotResponse {
	answer: string;
	suggested_sql: string | null;
	pipeline_suggestions: string[];
	ontology_hints: string[];
	cited_knowledge: KnowledgeSearchResult[];
	provider_name: string;
	cache: SemanticCacheMetadata;
	usage: LlmUsageSummary;
	created_at: string;
}

export interface EvaluateGuardrailsResponse {
	verdict: GuardrailVerdict;
	risk_score: number;
	recommendations: string[];
}

export interface ProviderBenchmarkScore {
	quality: number;
	latency: number;
	cost: number;
	safety: number;
	overall: number;
}

export interface ProviderBenchmarkResult {
	provider_id: string;
	provider_name: string;
	network_scope: string;
	reply_preview: string;
	prompt_tokens: number;
	completion_tokens: number;
	total_tokens: number;
	estimated_cost_usd: number;
	latency_ms: number;
	cache_hit: boolean;
	guardrail: GuardrailVerdict;
	score: ProviderBenchmarkScore;
	error: string | null;
}

export interface ProviderBenchmarkResponse {
	benchmark_group_id: string;
	use_case: string;
	prompt_excerpt: string;
	required_modalities: string[];
	requested_private_network: boolean;
	recommended_provider_id: string | null;
	results: ProviderBenchmarkResult[];
	created_at: string;
}

export interface AiPlatformOverview {
	provider_count: number;
	private_provider_count: number;
	multimodal_provider_count: number;
	prompt_count: number;
	knowledge_base_count: number;
	indexed_document_count: number;
	indexed_chunk_count: number;
	agent_count: number;
	conversation_count: number;
	cache_entry_count: number;
	cache_hit_rate: number;
	blocked_guardrail_events: number;
	llm_prompt_tokens: number;
	llm_completion_tokens: number;
	estimated_llm_cost_usd: number;
	benchmark_run_count: number;
}

export function getOverview() {
	return api.get<AiPlatformOverview>('/ai/overview');
}

export function listProviders() {
	return api.get<ListResponse<LlmProvider>>('/ai/providers');
}

export function createProvider(body: {
	name: string;
	provider_type: string;
	model_name: string;
	endpoint_url: string;
	api_mode?: string;
	credential_reference?: string;
	enabled?: boolean;
	load_balance_weight?: number;
	max_output_tokens?: number;
	cost_tier?: string;
	tags?: string[];
	route_rules?: Partial<ProviderRoutingRules>;
}) {
	return api.post<LlmProvider>('/ai/providers', body);
}

export function updateProvider(id: string, body: {
	name?: string;
	provider_type?: string;
	model_name?: string;
	endpoint_url?: string;
	api_mode?: string;
	credential_reference?: string;
	enabled?: boolean;
	load_balance_weight?: number;
	max_output_tokens?: number;
	cost_tier?: string;
	tags?: string[];
	route_rules?: ProviderRoutingRules;
	health_state?: ProviderHealthState;
}) {
	return api.patch<LlmProvider>(`/ai/providers/${id}`, body);
}

export function listPrompts() {
	return api.get<ListResponse<PromptTemplate>>('/ai/prompts');
}

export function createPrompt(body: {
	name: string;
	description?: string;
	category?: string;
	tags?: string[];
	content: string;
	input_variables?: string[];
	notes?: string;
}) {
	return api.post<PromptTemplate>('/ai/prompts', body);
}

export function updatePrompt(id: string, body: {
	name?: string;
	description?: string;
	category?: string;
	status?: string;
	tags?: string[];
	content?: string;
	input_variables?: string[];
	notes?: string;
}) {
	return api.patch<PromptTemplate>(`/ai/prompts/${id}`, body);
}

export function renderPrompt(id: string, body: {
	variables?: Record<string, string>;
	strict?: boolean;
}) {
	return api.post<RenderPromptResponse>(`/ai/prompts/${id}/render`, body);
}

export function listKnowledgeBases() {
	return api.get<ListResponse<KnowledgeBase>>('/ai/knowledge-bases');
}

export function createKnowledgeBase(body: {
	name: string;
	description?: string;
	status?: string;
	embedding_provider?: string;
	chunking_strategy?: string;
	tags?: string[];
}) {
	return api.post<KnowledgeBase>('/ai/knowledge-bases', body);
}

export function updateKnowledgeBase(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	embedding_provider?: string;
	chunking_strategy?: string;
	tags?: string[];
}) {
	return api.patch<KnowledgeBase>(`/ai/knowledge-bases/${id}`, body);
}

export function listKnowledgeDocuments(id: string) {
	return api.get<ListResponse<KnowledgeDocument>>(`/ai/knowledge-bases/${id}/documents`);
}

export function createKnowledgeDocument(id: string, body: {
	title: string;
	content: string;
	source_uri?: string;
	metadata?: Record<string, unknown>;
}) {
	return api.post<KnowledgeDocument>(`/ai/knowledge-bases/${id}/documents`, body);
}

export function searchKnowledgeBase(id: string, body: {
	query: string;
	top_k?: number;
	min_score?: number;
}) {
	return api.post<SearchKnowledgeBaseResponse>(`/ai/knowledge-bases/${id}/search`, body);
}

export function listTools() {
	return api.get<ListResponse<ToolDefinition>>('/ai/tools');
}

export function createTool(body: {
	name: string;
	description?: string;
	category?: string;
	execution_mode?: string;
	execution_config?: Record<string, unknown>;
	status?: string;
	input_schema?: Record<string, unknown>;
	output_schema?: Record<string, unknown>;
	tags?: string[];
}) {
	return api.post<ToolDefinition>('/ai/tools', body);
}

export function updateTool(id: string, body: {
	name?: string;
	description?: string;
	category?: string;
	execution_mode?: string;
	execution_config?: Record<string, unknown>;
	status?: string;
	input_schema?: Record<string, unknown>;
	output_schema?: Record<string, unknown>;
	tags?: string[];
}) {
	return api.patch<ToolDefinition>(`/ai/tools/${id}`, body);
}

export function listAgents() {
	return api.get<ListResponse<AgentDefinition>>('/ai/agents');
}

export function createAgent(body: {
	name: string;
	description?: string;
	status?: string;
	system_prompt?: string;
	objective?: string;
	tool_ids?: string[];
	planning_strategy?: string;
	max_iterations?: number;
}) {
	return api.post<AgentDefinition>('/ai/agents', body);
}

export function updateAgent(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	system_prompt?: string;
	objective?: string;
	tool_ids?: string[];
	planning_strategy?: string;
	max_iterations?: number;
	memory?: AgentMemorySnapshot;
}) {
	return api.patch<AgentDefinition>(`/ai/agents/${id}`, body);
}

export function executeAgent(id: string, body: {
	user_message: string;
	objective?: string;
	knowledge_base_id?: string;
	context?: Record<string, unknown>;
}) {
	return api.post<AgentExecutionResponse>(`/ai/agents/${id}/execute`, body);
}

export function listConversations() {
	return api.get<ListResponse<ConversationSummary>>('/ai/conversations');
}

export function getConversation(id: string) {
	return api.get<Conversation>(`/ai/conversations/${id}`);
}

export function createChatCompletion(body: {
	conversation_id?: string;
	user_message: string;
	system_prompt?: string;
	prompt_template_id?: string;
	prompt_variables?: Record<string, string>;
	knowledge_base_id?: string;
	preferred_provider_id?: string;
	attachments?: ChatAttachment[];
	max_tokens?: number;
	fallback_enabled?: boolean;
	require_private_network?: boolean;
}) {
	return api.post<ChatCompletionResponse>('/ai/chat/completions', body);
}

export function runProviderBenchmark(body: {
	prompt: string;
	system_prompt?: string;
	provider_ids?: string[];
	attachments?: ChatAttachment[];
	rubric_keywords?: string[];
	use_case?: string;
	max_tokens?: number;
	require_private_network?: boolean;
}) {
	return api.post<ProviderBenchmarkResponse>('/ai/evaluations/benchmarks', body);
}

export function askCopilot(body: {
	question: string;
	dataset_ids?: string[];
	ontology_type_ids?: string[];
	knowledge_base_ids?: string[];
	include_sql?: boolean;
	include_pipeline_plan?: boolean;
	preferred_provider_id?: string;
}) {
	return api.post<CopilotResponse>('/ai/copilot/ask', body);
}

export function evaluateGuardrails(body: {
	content: string;
}) {
	return api.post<EvaluateGuardrailsResponse>('/ai/guardrails/evaluate', body);
}
