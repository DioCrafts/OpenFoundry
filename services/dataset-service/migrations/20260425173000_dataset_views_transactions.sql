CREATE TABLE IF NOT EXISTS dataset_views (
    id UUID PRIMARY KEY,
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    sql_text TEXT NOT NULL,
    source_branch TEXT,
    source_version INT,
    materialized BOOLEAN NOT NULL DEFAULT TRUE,
    refresh_on_source_update BOOLEAN NOT NULL DEFAULT FALSE,
    format TEXT NOT NULL DEFAULT 'json',
    current_version INT NOT NULL DEFAULT 0,
    storage_path TEXT,
    row_count BIGINT NOT NULL DEFAULT 0,
    schema_fields JSONB NOT NULL DEFAULT '[]',
    last_refreshed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(dataset_id, name)
);

CREATE INDEX IF NOT EXISTS idx_dataset_views_dataset
    ON dataset_views(dataset_id, created_at DESC);

CREATE TABLE IF NOT EXISTS dataset_transactions (
    id UUID PRIMARY KEY,
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    view_id UUID REFERENCES dataset_views(id) ON DELETE SET NULL,
    operation TEXT NOT NULL,
    branch_name TEXT,
    status TEXT NOT NULL DEFAULT 'committed',
    summary TEXT NOT NULL DEFAULT '',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    committed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_dataset_transactions_dataset_created
    ON dataset_transactions(dataset_id, created_at DESC);

ALTER TABLE dataset_versions
    ADD COLUMN IF NOT EXISTS transaction_id UUID REFERENCES dataset_transactions(id) ON DELETE SET NULL;
