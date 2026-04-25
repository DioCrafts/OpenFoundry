CREATE TABLE IF NOT EXISTS ontology_quiver_visual_functions (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    primary_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    secondary_type_id UUID REFERENCES object_types(id) ON DELETE SET NULL,
    join_field TEXT NOT NULL,
    secondary_join_field TEXT NOT NULL DEFAULT '',
    date_field TEXT NOT NULL,
    metric_field TEXT NOT NULL,
    group_field TEXT NOT NULL,
    selected_group TEXT,
    chart_kind TEXT NOT NULL DEFAULT 'line',
    shared BOOLEAN NOT NULL DEFAULT FALSE,
    vega_spec JSONB NOT NULL DEFAULT '{}'::jsonb,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT quiver_visual_chart_kind_valid CHECK (chart_kind IN ('line', 'area', 'bar', 'point'))
);

CREATE INDEX IF NOT EXISTS idx_quiver_visual_functions_owner
    ON ontology_quiver_visual_functions(owner_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_quiver_visual_functions_shared
    ON ontology_quiver_visual_functions(shared, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_quiver_visual_functions_primary_type
    ON ontology_quiver_visual_functions(primary_type_id);
