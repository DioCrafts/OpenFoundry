CREATE TABLE IF NOT EXISTS ai_providers (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    model_name TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    api_mode TEXT NOT NULL DEFAULT 'chat_completions',
    credential_reference TEXT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    load_balance_weight INTEGER NOT NULL DEFAULT 100,
    max_output_tokens INTEGER NOT NULL DEFAULT 2048,
    cost_tier TEXT NOT NULL DEFAULT 'standard',
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    route_rules JSONB NOT NULL DEFAULT '{}'::jsonb,
    health_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_prompt_templates (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'copilot',
    status TEXT NOT NULL DEFAULT 'active',
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    latest_version_number INTEGER NOT NULL DEFAULT 1,
    versions JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_knowledge_bases (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    embedding_provider TEXT NOT NULL DEFAULT 'deterministic-hash',
    chunking_strategy TEXT NOT NULL DEFAULT 'balanced',
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    document_count BIGINT NOT NULL DEFAULT 0,
    chunk_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_knowledge_documents (
    id UUID PRIMARY KEY,
    knowledge_base_id UUID NOT NULL REFERENCES ai_knowledge_bases(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    source_uri TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'indexed',
    chunk_count INTEGER NOT NULL DEFAULT 0,
    chunks JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_tools (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'analysis',
    execution_mode TEXT NOT NULL DEFAULT 'simulated',
    status TEXT NOT NULL DEFAULT 'active',
    input_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_agents (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    system_prompt TEXT NOT NULL DEFAULT '',
    objective TEXT NOT NULL DEFAULT '',
    tool_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    planning_strategy TEXT NOT NULL DEFAULT 'plan-act-observe',
    max_iterations INTEGER NOT NULL DEFAULT 3,
    memory JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_execution_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_conversations (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    messages JSONB NOT NULL DEFAULT '[]'::jsonb,
    provider_id UUID NULL REFERENCES ai_providers(id) ON DELETE SET NULL,
    last_cache_hit BOOLEAN NOT NULL DEFAULT FALSE,
    last_guardrail_blocked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ai_semantic_cache (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    cache_key TEXT NOT NULL,
    normalized_prompt TEXT NOT NULL,
    fingerprint JSONB NOT NULL DEFAULT '[]'::jsonb,
    response JSONB NOT NULL DEFAULT '{}'::jsonb,
    provider_id UUID NULL REFERENCES ai_providers(id) ON DELETE SET NULL,
    hit_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_hit_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(kind, cache_key)
);

CREATE INDEX IF NOT EXISTS idx_ai_prompt_templates_status ON ai_prompt_templates(status);
CREATE INDEX IF NOT EXISTS idx_ai_knowledge_documents_kb_id ON ai_knowledge_documents(knowledge_base_id);
CREATE INDEX IF NOT EXISTS idx_ai_agents_status ON ai_agents(status);
CREATE INDEX IF NOT EXISTS idx_ai_conversations_last_activity ON ai_conversations(last_activity_at DESC);
CREATE INDEX IF NOT EXISTS idx_ai_semantic_cache_kind ON ai_semantic_cache(kind);

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
) VALUES (
    '01967f76-3550-70c0-9000-000000000001',
    'OpenAI Primary',
    'openai',
    'gpt-4.1-mini',
    'https://api.openai.com/v1',
    'chat_completions',
    'OPENAI_API_KEY',
    TRUE,
    120,
    4096,
    'standard',
    '["production", "chat"]'::jsonb,
    '{"use_cases":["chat","copilot","general"],"preferred_regions":["eu-west-1"],"fallback_provider_ids":["01967f76-3550-70c0-9000-000000000002","01967f76-3550-70c0-9000-000000000003"],"weight":120,"max_context_tokens":64000}'::jsonb,
    '{"status":"healthy","avg_latency_ms":540,"error_rate":0.01,"last_checked_at":"2026-04-22T12:00:00Z"}'::jsonb
) ON CONFLICT (id) DO NOTHING;

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
) VALUES (
    '01967f76-3550-70c0-9000-000000000002',
    'Anthropic Fallback',
    'anthropic',
    'claude-3.7-sonnet',
    'https://api.anthropic.com/v1',
    'messages',
    'ANTHROPIC_API_KEY',
    TRUE,
    90,
    4096,
    'premium',
    '["fallback", "copilot"]'::jsonb,
    '{"use_cases":["copilot","agents","general"],"preferred_regions":["us-east-1"],"fallback_provider_ids":["01967f76-3550-70c0-9000-000000000003"],"weight":90,"max_context_tokens":200000}'::jsonb,
    '{"status":"healthy","avg_latency_ms":690,"error_rate":0.02,"last_checked_at":"2026-04-22T12:00:00Z"}'::jsonb
) ON CONFLICT (id) DO NOTHING;

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
) VALUES (
    '01967f76-3550-70c0-9000-000000000003',
    'Local Ollama',
    'ollama',
    'llama3.1:8b',
    'http://localhost:11434/api',
    'chat',
    NULL,
    TRUE,
    40,
    2048,
    'local',
    '["local", "rag"]'::jsonb,
    '{"use_cases":["rag","general"],"preferred_regions":[],"fallback_provider_ids":[],"weight":40,"max_context_tokens":8192}'::jsonb,
    '{"status":"degraded","avg_latency_ms":910,"error_rate":0.05,"last_checked_at":"2026-04-22T12:00:00Z"}'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_prompt_templates (
    id,
    name,
    description,
    category,
    status,
    tags,
    latest_version_number,
    versions
) VALUES (
    '01967f76-3550-70c0-9000-000000000010',
    'Operations Copilot',
    'Default prompt for operational SQL and workflow assistance.',
    'copilot',
    'active',
    '["copilot", "operations"]'::jsonb,
    1,
    '[{"version_number":1,"content":"You are OpenFoundry Copilot for {{team_name}}. Ground your answer in platform data, suggest SQL when useful, and keep outputs concise.","input_variables":["team_name"],"notes":"Initial copilot prompt","created_at":"2026-04-22T12:00:00Z","created_by":null}]'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_knowledge_bases (
    id,
    name,
    description,
    status,
    embedding_provider,
    chunking_strategy,
    tags,
    document_count,
    chunk_count
) VALUES (
    '01967f76-3550-70c0-9000-000000000020',
    'Platform Playbooks',
    'Runbooks and operating procedures for common platform incidents.',
    'active',
    'deterministic-hash',
    'balanced',
    '["runbooks", "support"]'::jsonb,
    1,
    2
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_knowledge_documents (
    id,
    knowledge_base_id,
    title,
    content,
    source_uri,
    metadata,
    status,
    chunk_count,
    chunks
) VALUES (
    '01967f76-3550-70c0-9000-000000000021',
    '01967f76-3550-70c0-9000-000000000020',
    'Incident Triage Checklist',
    'Confirm the affected workspace and gather query ids before escalating. If latency exceeds 500ms for three consecutive checks, reroute to a fallback provider and capture the request id. For retrieval issues, re-index the knowledge base and compare chunk counts to the last successful run.',
    'kb://platform-playbooks/incident-triage',
    '{"owner":"platform-ops","tier":"critical"}'::jsonb,
    'indexed',
    2,
    '[{"id":"01967f76-3550-70c0-9000-000000000021-0","position":0,"text":"Confirm the affected workspace and gather query ids before escalating. If latency exceeds 500ms for three consecutive checks, reroute to a fallback provider and capture the request id.","token_count":27,"embedding":[0.42,0.11,0.09,0.18,0.28,0.17,0.21,0.13,0.08,0.19,0.24,0.14],"metadata":{"strategy":"balanced"}},{"id":"01967f76-3550-70c0-9000-000000000021-1","position":1,"text":"For retrieval issues, re-index the knowledge base and compare chunk counts to the last successful run.","token_count":15,"embedding":[0.18,0.31,0.27,0.12,0.09,0.22,0.16,0.11,0.29,0.14,0.17,0.26],"metadata":{"strategy":"balanced"}}]'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_tools (
    id,
    name,
    description,
    category,
    execution_mode,
    status,
    input_schema,
    output_schema,
    tags
) VALUES (
    '01967f76-3550-70c0-9000-000000000030',
    'SQL Generator',
    'Creates starter SQL from natural language intents.',
    'analysis',
    'simulated',
    'active',
    '{"type":"object","properties":{"question":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"sql":{"type":"string"}}}'::jsonb,
    '["sql", "copilot"]'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_tools (
    id,
    name,
    description,
    category,
    execution_mode,
    status,
    input_schema,
    output_schema,
    tags
) VALUES (
    '01967f76-3550-70c0-9000-000000000031',
    'Ontology Mapper',
    'Suggests ontology types and link mappings for generated outputs.',
    'reasoning',
    'simulated',
    'active',
    '{"type":"object","properties":{"answer":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"ontology_hints":{"type":"array"}}}'::jsonb,
    '["ontology", "agents"]'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_agents (
    id,
    name,
    description,
    status,
    system_prompt,
    objective,
    tool_ids,
    planning_strategy,
    max_iterations,
    memory,
    last_execution_at
) VALUES (
    '01967f76-3550-70c0-9000-000000000040',
    'Platform Analyst',
    'Investigates operational questions using prompt, SQL, and ontology tools.',
    'active',
    'Use platform context first, then propose SQL or workflow actions.',
    'Help operators resolve platform incidents quickly and with traceability.',
    '["01967f76-3550-70c0-9000-000000000030","01967f76-3550-70c0-9000-000000000031"]'::jsonb,
    'plan-act-observe',
    3,
    '{"short_term_notes":["Monitor provider latency regressions daily."],"long_term_references":["Incident Triage Checklist"],"last_run_summary":"Seeded platform analyst agent."}'::jsonb,
    NOW() - INTERVAL '3 hours'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_conversations (
    id,
    title,
    messages,
    provider_id,
    last_cache_hit,
    last_guardrail_blocked
) VALUES (
    '01967f76-3550-70c0-9000-000000000050',
    'How do I reroute an overloaded provider?',
    '[{"role":"user","content":"How do I reroute an overloaded provider?","provider_id":null,"tool_name":null,"citations":[],"guardrail_verdict":{"status":"passed","redacted_text":"How do I reroute an overloaded provider?","blocked":false,"flags":[]},"created_at":"2026-04-22T12:00:00Z"},{"role":"assistant","content":"Use the incident playbook, compare latency against the 500ms threshold, and update the provider priority order before retrying.","provider_id":"01967f76-3550-70c0-9000-000000000001","tool_name":null,"citations":[{"knowledge_base_id":"01967f76-3550-70c0-9000-000000000020","document_id":"01967f76-3550-70c0-9000-000000000021","document_title":"Incident Triage Checklist","chunk_id":"01967f76-3550-70c0-9000-000000000021-0","score":0.94,"excerpt":"Confirm the affected workspace and gather query ids before escalating. If latency exceeds 500ms for three consecutive checks, reroute to a fallback provider and capture the request id.","source_uri":"kb://platform-playbooks/incident-triage","metadata":{"strategy":"balanced"}}],"guardrail_verdict":null,"created_at":"2026-04-22T12:00:30Z"}]'::jsonb,
    '01967f76-3550-70c0-9000-000000000001',
    TRUE,
    FALSE
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ai_semantic_cache (
    id,
    kind,
    cache_key,
    normalized_prompt,
    fingerprint,
    response,
    provider_id,
    hit_count,
    created_at,
    last_hit_at
) VALUES (
    '01967f76-3550-70c0-9000-000000000060',
    'copilot',
    'copilot:how do i reroute an overloaded provider datasets ontology knowledge bases',
    'how do i reroute an overloaded provider datasets ontology knowledge bases',
    '[0.28,0.17,0.11,0.19,0.23,0.14,0.09,0.31,0.16,0.21,0.12,0.22,0.18,0.07,0.24,0.13]'::jsonb,
    '{"answer":"Start with the incident triage checklist, verify latency against the fallback threshold, and preserve the request id for auditability.","suggested_sql":"SELECT provider_name, avg_latency_ms, error_rate FROM provider_metrics ORDER BY avg_latency_ms DESC LIMIT 20;","pipeline_suggestions":["Schedule provider health snapshots every 5 minutes.","Trigger a notification when latency crosses the threshold three times in a row."],"ontology_hints":["Map provider incidents to an Incident object and connect the affected Provider object."],"cited_knowledge":[{"knowledge_base_id":"01967f76-3550-70c0-9000-000000000020","document_id":"01967f76-3550-70c0-9000-000000000021","document_title":"Incident Triage Checklist","chunk_id":"01967f76-3550-70c0-9000-000000000021-0","score":0.94,"excerpt":"Confirm the affected workspace and gather query ids before escalating. If latency exceeds 500ms for three consecutive checks, reroute to a fallback provider and capture the request id.","source_uri":"kb://platform-playbooks/incident-triage","metadata":{"strategy":"balanced"}}]}'::jsonb,
    '01967f76-3550-70c0-9000-000000000001',
    4,
    NOW() - INTERVAL '2 hours',
    NOW() - INTERVAL '10 minutes'
) ON CONFLICT (kind, cache_key) DO NOTHING;