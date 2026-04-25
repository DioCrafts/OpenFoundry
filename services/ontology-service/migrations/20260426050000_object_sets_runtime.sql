CREATE TABLE IF NOT EXISTS ontology_object_sets (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    base_object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    filters JSONB NOT NULL DEFAULT '[]'::jsonb,
    traversals JSONB NOT NULL DEFAULT '[]'::jsonb,
    join_config JSONB,
    projections JSONB NOT NULL DEFAULT '[]'::jsonb,
    what_if_label TEXT,
    policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    materialized_snapshot JSONB,
    materialized_at TIMESTAMPTZ,
    materialized_row_count INTEGER NOT NULL DEFAULT 0,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ontology_object_sets_owner
    ON ontology_object_sets(owner_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_ontology_object_sets_base_type
    ON ontology_object_sets(base_object_type_id, updated_at DESC);
