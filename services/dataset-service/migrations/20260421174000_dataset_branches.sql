ALTER TABLE datasets
    ADD COLUMN IF NOT EXISTS active_branch TEXT NOT NULL DEFAULT 'main';

CREATE TABLE IF NOT EXISTS dataset_branches (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     INT NOT NULL DEFAULT 1,
    description TEXT NOT NULL DEFAULT '',
    is_default  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(dataset_id, name)
);

CREATE INDEX IF NOT EXISTS idx_dataset_branches_dataset ON dataset_branches(dataset_id);