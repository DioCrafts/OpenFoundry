CREATE TABLE IF NOT EXISTS notebooks (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner_id    UUID NOT NULL,
    default_kernel TEXT NOT NULL DEFAULT 'python',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cells (
    id              UUID PRIMARY KEY,
    notebook_id     UUID NOT NULL REFERENCES notebooks(id) ON DELETE CASCADE,
    cell_type       TEXT NOT NULL DEFAULT 'code',
    kernel          TEXT NOT NULL DEFAULT 'python',
    source          TEXT NOT NULL DEFAULT '',
    position        INT NOT NULL DEFAULT 0,
    last_output     JSONB,
    execution_count INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cells_notebook ON cells(notebook_id, position);

CREATE TABLE IF NOT EXISTS sessions (
    id              UUID PRIMARY KEY,
    notebook_id     UUID NOT NULL REFERENCES notebooks(id) ON DELETE CASCADE,
    kernel          TEXT NOT NULL DEFAULT 'python',
    status          TEXT NOT NULL DEFAULT 'idle',
    started_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_notebook ON sessions(notebook_id);
