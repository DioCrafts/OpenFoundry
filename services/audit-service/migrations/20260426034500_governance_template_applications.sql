CREATE TABLE IF NOT EXISTS governance_template_applications (
    id UUID PRIMARY KEY,
    template_slug TEXT NOT NULL,
    template_name TEXT NOT NULL,
    scope TEXT NOT NULL,
    standards JSONB NOT NULL DEFAULT '[]'::jsonb,
    policy_names JSONB NOT NULL DEFAULT '[]'::jsonb,
    checkpoint_prompts JSONB NOT NULL DEFAULT '[]'::jsonb,
    sds_remediations JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_report_standard TEXT NOT NULL,
    applied_by TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT governance_template_applications_scope_unique UNIQUE (template_slug, scope)
);

CREATE INDEX IF NOT EXISTS idx_governance_template_applications_scope
    ON governance_template_applications (scope, updated_at DESC);
