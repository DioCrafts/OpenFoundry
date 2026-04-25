CREATE TABLE IF NOT EXISTS ontology_function_packages (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    runtime TEXT NOT NULL,
    source TEXT NOT NULL,
    entrypoint TEXT NOT NULL DEFAULT 'handler',
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ontology_function_packages_runtime
    ON ontology_function_packages(runtime);

CREATE TABLE IF NOT EXISTS ontology_rules (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    evaluation_mode TEXT NOT NULL DEFAULT 'advisory',
    trigger_spec JSONB NOT NULL DEFAULT '{}'::jsonb,
    effect_spec JSONB NOT NULL DEFAULT '{}'::jsonb,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ontology_rules_object_type
    ON ontology_rules(object_type_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rules_evaluation_mode
    ON ontology_rules(evaluation_mode);

CREATE TABLE IF NOT EXISTS ontology_rule_runs (
    id UUID PRIMARY KEY,
    rule_id UUID NOT NULL REFERENCES ontology_rules(id) ON DELETE CASCADE,
    object_id UUID NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    matched BOOLEAN NOT NULL DEFAULT FALSE,
    simulated BOOLEAN NOT NULL DEFAULT FALSE,
    trigger_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    effect_preview JSONB,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_runs_rule_id
    ON ontology_rule_runs(rule_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_runs_object_id
    ON ontology_rule_runs(object_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_runs_created_at
    ON ontology_rule_runs(created_at DESC);
