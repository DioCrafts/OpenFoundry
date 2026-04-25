CREATE TABLE IF NOT EXISTS ai_llm_usage_events (
    id UUID PRIMARY KEY,
    provider_id UUID NULL REFERENCES ai_providers(id) ON DELETE SET NULL,
    conversation_id UUID NULL REFERENCES ai_conversations(id) ON DELETE SET NULL,
    request_kind TEXT NOT NULL DEFAULT 'chat',
    use_case TEXT NOT NULL DEFAULT 'chat',
    network_scope TEXT NOT NULL DEFAULT 'public',
    modality TEXT NOT NULL DEFAULT 'text',
    cache_hit BOOLEAN NOT NULL DEFAULT FALSE,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    latency_ms INTEGER NOT NULL DEFAULT 0,
    benchmark_group_id UUID NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_llm_usage_events_provider_created
    ON ai_llm_usage_events(provider_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_ai_llm_usage_events_benchmark_group
    ON ai_llm_usage_events(benchmark_group_id)
    WHERE benchmark_group_id IS NOT NULL;

UPDATE ai_providers
SET route_rules = route_rules
    || jsonb_build_object(
        'network_scope', 'public',
        'supported_modalities', '["text","image","embedding"]'::jsonb,
        'input_cost_per_1k_tokens_usd', 0.00015,
        'output_cost_per_1k_tokens_usd', 0.00060
    )
WHERE id = '01967f76-3550-70c0-9000-000000000001';

UPDATE ai_providers
SET route_rules = route_rules
    || jsonb_build_object(
        'network_scope', 'public',
        'supported_modalities', '["text","image"]'::jsonb,
        'input_cost_per_1k_tokens_usd', 0.00300,
        'output_cost_per_1k_tokens_usd', 0.01500
    )
WHERE id = '01967f76-3550-70c0-9000-000000000002';

UPDATE ai_providers
SET route_rules = route_rules
    || jsonb_build_object(
        'network_scope', 'local',
        'supported_modalities', '["text","image","embedding"]'::jsonb,
        'input_cost_per_1k_tokens_usd', 0.0,
        'output_cost_per_1k_tokens_usd', 0.0
    )
WHERE id = '01967f76-3550-70c0-9000-000000000003';
