-- Pipelines, runs, and lineage

CREATE TABLE IF NOT EXISTS pipelines (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner_id    UUID NOT NULL,
    dag         JSONB NOT NULL DEFAULT '[]',
    status      TEXT NOT NULL DEFAULT 'draft',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pipeline_runs (
    id           UUID PRIMARY KEY,
    pipeline_id  UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    status       TEXT NOT NULL DEFAULT 'pending',
    started_by   UUID NOT NULL,
    node_results JSONB,
    error_message TEXT,
    started_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at  TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS lineage_edges (
    id                UUID PRIMARY KEY,
    source_dataset_id UUID NOT NULL,
    target_dataset_id UUID NOT NULL,
    pipeline_id       UUID REFERENCES pipelines(id) ON DELETE SET NULL,
    node_id           TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_dataset_id, target_dataset_id, pipeline_id)
);

CREATE INDEX idx_pipeline_runs_pipeline ON pipeline_runs(pipeline_id);
CREATE INDEX idx_lineage_source ON lineage_edges(source_dataset_id);
CREATE INDEX idx_lineage_target ON lineage_edges(target_dataset_id);
