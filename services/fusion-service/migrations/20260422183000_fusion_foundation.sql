CREATE TABLE IF NOT EXISTS fusion_match_rules (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    entity_type TEXT NOT NULL DEFAULT 'person',
    blocking_strategy JSONB NOT NULL DEFAULT '{}'::jsonb,
    conditions JSONB NOT NULL DEFAULT '[]'::jsonb,
    review_threshold REAL NOT NULL DEFAULT 0.76,
    auto_merge_threshold REAL NOT NULL DEFAULT 0.90,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS fusion_merge_strategies (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    entity_type TEXT NOT NULL DEFAULT 'person',
    default_strategy TEXT NOT NULL DEFAULT 'longest_non_empty',
    rules JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS fusion_jobs (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft',
    entity_type TEXT NOT NULL DEFAULT 'person',
    match_rule_id UUID NOT NULL REFERENCES fusion_match_rules(id) ON DELETE RESTRICT,
    merge_strategy_id UUID NOT NULL REFERENCES fusion_merge_strategies(id) ON DELETE RESTRICT,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_run_summary TEXT NOT NULL DEFAULT 'Not run yet',
    last_run_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS fusion_clusters (
    id UUID PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES fusion_jobs(id) ON DELETE CASCADE,
    cluster_key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'singleton',
    records JSONB NOT NULL DEFAULT '[]'::jsonb,
    evidence JSONB NOT NULL DEFAULT '[]'::jsonb,
    confidence_score REAL NOT NULL DEFAULT 0.0,
    requires_review BOOLEAN NOT NULL DEFAULT FALSE,
    suggested_golden_record_id UUID NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS fusion_review_queue (
    id UUID PRIMARY KEY,
    cluster_id UUID NOT NULL REFERENCES fusion_clusters(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    severity TEXT NOT NULL DEFAULT 'medium',
    recommended_action TEXT NOT NULL DEFAULT 'manual_review',
    rationale JSONB NOT NULL DEFAULT '[]'::jsonb,
    assigned_to TEXT NULL,
    reviewed_by TEXT NULL,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS fusion_golden_records (
    id UUID PRIMARY KEY,
    cluster_id UUID NOT NULL REFERENCES fusion_clusters(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    canonical_values JSONB NOT NULL DEFAULT '{}'::jsonb,
    provenance JSONB NOT NULL DEFAULT '[]'::jsonb,
    completeness_score REAL NOT NULL DEFAULT 0.0,
    confidence_score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fusion_jobs_status ON fusion_jobs(status);
CREATE INDEX IF NOT EXISTS idx_fusion_clusters_job_id ON fusion_clusters(job_id);
CREATE INDEX IF NOT EXISTS idx_fusion_review_queue_status ON fusion_review_queue(status);
CREATE INDEX IF NOT EXISTS idx_fusion_golden_records_cluster_id ON fusion_golden_records(cluster_id);

INSERT INTO fusion_match_rules (
    id,
    name,
    description,
    status,
    entity_type,
    blocking_strategy,
    conditions,
    review_threshold,
    auto_merge_threshold
) VALUES (
    '01967fd8-0850-7920-9000-000000000001',
    'Person Resolution Rule',
    'Balances exact email and phone matches with fuzzy and phonetic name comparison.',
    'active',
    'person',
    '{"strategy_type":"sorted-neighborhood","key_fields":["email","phone","display_name"],"window_size":4,"bucket_count":24}'::jsonb,
    '[
        {"field":"email","comparator":"email_exact","weight":0.35,"threshold":1.0,"required":false},
        {"field":"phone","comparator":"phone_exact","weight":0.20,"threshold":1.0,"required":false},
        {"field":"display_name","comparator":"jaro_winkler","weight":0.25,"threshold":0.86,"required":true},
        {"field":"display_name","comparator":"phonetic","weight":0.10,"threshold":0.50,"required":false},
        {"field":"company","comparator":"fuzzy","weight":0.10,"threshold":0.72,"required":false}
    ]'::jsonb,
    0.76,
    0.90
) ON CONFLICT (id) DO NOTHING;

INSERT INTO fusion_merge_strategies (
    id,
    name,
    description,
    status,
    entity_type,
    default_strategy,
    rules
) VALUES (
    '01967fd8-0850-7920-9000-000000000010',
    'Person Survivorship',
    'Prioritize CRM for emails, use longest names, and consolidate phones by consensus.',
    'active',
    'person',
    'longest_non_empty',
    '[
        {"field":"display_name","strategy":"longest_non_empty","source_priority":["crm","erp","support"],"fallback":"highest_confidence"},
        {"field":"email","strategy":"source_priority","source_priority":["crm","erp","support"],"fallback":"most_common"},
        {"field":"phone","strategy":"most_common","source_priority":[],"fallback":"longest_non_empty"},
        {"field":"company","strategy":"most_common","source_priority":[],"fallback":"longest_non_empty"},
        {"field":"city","strategy":"most_common","source_priority":[],"fallback":"longest_non_empty"}
    ]'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO fusion_jobs (
    id,
    name,
    description,
    status,
    entity_type,
    match_rule_id,
    merge_strategy_id,
    config,
    metrics,
    last_run_summary,
    last_run_at
) VALUES (
    '01967fd8-0850-7920-9000-000000000020',
    'Customer 360 Batch',
    'Resolve customer identities across CRM, ERP, and support exports.',
    'draft',
    'person',
    '01967fd8-0850-7920-9000-000000000001',
    '01967fd8-0850-7920-9000-000000000010',
    '{"source_labels":["crm","erp","support"],"record_count":12,"blocking_strategy_override":null,"review_sampling_rate":0.25}'::jsonb,
    '{"candidate_pairs":0,"matched_pairs":0,"review_pairs":0,"cluster_count":0,"golden_record_count":0,"precision_estimate":0.0,"recall_estimate":0.0}'::jsonb,
    'Seeded job ready to run deterministic Fusion resolution.',
    NULL
) ON CONFLICT (id) DO NOTHING;