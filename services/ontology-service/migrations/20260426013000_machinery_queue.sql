CREATE TABLE IF NOT EXISTS ontology_rule_schedules (
    id UUID PRIMARY KEY,
    rule_id UUID NOT NULL REFERENCES ontology_rules(id) ON DELETE CASCADE,
    rule_run_id UUID NOT NULL REFERENCES ontology_rule_runs(id) ON DELETE CASCADE,
    object_id UUID NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    scheduled_for TIMESTAMPTZ NOT NULL,
    priority_score INTEGER NOT NULL DEFAULT 50,
    estimated_duration_minutes INTEGER NOT NULL DEFAULT 30,
    required_capability TEXT NULL,
    constraint_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_schedules_rule_id
    ON ontology_rule_schedules(rule_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_schedules_rule_run_id
    ON ontology_rule_schedules(rule_run_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_schedules_object_id
    ON ontology_rule_schedules(object_id);

CREATE INDEX IF NOT EXISTS idx_ontology_rule_schedules_status_scheduled_for
    ON ontology_rule_schedules(status, scheduled_for ASC);
