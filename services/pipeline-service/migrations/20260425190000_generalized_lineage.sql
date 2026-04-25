CREATE TABLE IF NOT EXISTS lineage_nodes (
    entity_id   UUID NOT NULL,
    entity_kind TEXT NOT NULL,
    label       TEXT NOT NULL,
    marking     TEXT NOT NULL DEFAULT 'public',
    metadata    JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, entity_kind)
);

CREATE TABLE IF NOT EXISTS lineage_relations (
    id                UUID PRIMARY KEY,
    source_id         UUID NOT NULL,
    source_kind       TEXT NOT NULL,
    target_id         UUID NOT NULL,
    target_kind       TEXT NOT NULL,
    relation_kind     TEXT NOT NULL,
    producer_key      TEXT NOT NULL,
    pipeline_id       UUID,
    workflow_id       UUID,
    node_id           TEXT,
    step_id           TEXT,
    effective_marking TEXT NOT NULL DEFAULT 'public',
    metadata          JSONB NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_id, source_kind, target_id, target_kind, relation_kind, producer_key)
);

CREATE INDEX IF NOT EXISTS idx_lineage_nodes_kind ON lineage_nodes(entity_kind, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_lineage_relations_source ON lineage_relations(source_kind, source_id);
CREATE INDEX IF NOT EXISTS idx_lineage_relations_target ON lineage_relations(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_lineage_relations_workflow ON lineage_relations(workflow_id);
CREATE INDEX IF NOT EXISTS idx_lineage_relations_pipeline ON lineage_relations(pipeline_id);
