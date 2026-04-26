CREATE TABLE IF NOT EXISTS ontology_funnel_sources (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL,
    pipeline_id UUID,
    dataset_branch TEXT,
    dataset_version INTEGER,
    preview_limit INTEGER NOT NULL DEFAULT 500,
    default_marking TEXT NOT NULL DEFAULT 'public',
    status TEXT NOT NULL DEFAULT 'active',
    property_mappings JSONB NOT NULL DEFAULT '[]'::jsonb,
    trigger_context JSONB NOT NULL DEFAULT '{}'::jsonb,
    owner_id UUID NOT NULL,
    last_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ontology_funnel_sources_object_type
    ON ontology_funnel_sources(object_type_id);
CREATE INDEX IF NOT EXISTS idx_ontology_funnel_sources_dataset
    ON ontology_funnel_sources(dataset_id);
CREATE INDEX IF NOT EXISTS idx_ontology_funnel_sources_pipeline
    ON ontology_funnel_sources(pipeline_id);
CREATE INDEX IF NOT EXISTS idx_ontology_funnel_sources_owner
    ON ontology_funnel_sources(owner_id);
CREATE INDEX IF NOT EXISTS idx_ontology_funnel_sources_status
    ON ontology_funnel_sources(status);

CREATE TABLE IF NOT EXISTS ontology_funnel_runs (
    id UUID PRIMARY KEY,
    source_id UUID NOT NULL REFERENCES ontology_funnel_sources(id) ON DELETE CASCADE,
    object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL,
    pipeline_id UUID,
    pipeline_run_id UUID,
    status TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    started_by UUID,
    rows_read INTEGER NOT NULL DEFAULT 0,
    inserted_count INTEGER NOT NULL DEFAULT 0,
    updated_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_ontology_funnel_runs_source
    ON ontology_funnel_runs(source_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_ontology_funnel_runs_object_type
    ON ontology_funnel_runs(object_type_id, started_at DESC);
