-- Dataset service schema

CREATE TABLE IF NOT EXISTS datasets (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    format      TEXT NOT NULL DEFAULT 'parquet',
    storage_path TEXT NOT NULL,
    size_bytes  BIGINT NOT NULL DEFAULT 0,
    row_count   BIGINT NOT NULL DEFAULT 0,
    owner_id    UUID NOT NULL,
    tags        TEXT[] NOT NULL DEFAULT '{}',
    current_version INT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_datasets_owner ON datasets(owner_id);
CREATE INDEX idx_datasets_name ON datasets(name);
CREATE INDEX idx_datasets_tags ON datasets USING GIN(tags);

CREATE TABLE IF NOT EXISTS dataset_schemas (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    fields      JSONB NOT NULL DEFAULT '[]',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_dataset_schemas_dataset ON dataset_schemas(dataset_id);

CREATE TABLE IF NOT EXISTS dataset_versions (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    version     INT NOT NULL,
    message     TEXT NOT NULL DEFAULT '',
    size_bytes  BIGINT NOT NULL DEFAULT 0,
    row_count   BIGINT NOT NULL DEFAULT 0,
    storage_path TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(dataset_id, version)
);

CREATE INDEX idx_versions_dataset ON dataset_versions(dataset_id);
