-- Data connector service schema

CREATE TABLE IF NOT EXISTS connections (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL,
    connector_type  TEXT NOT NULL,
    config          JSONB NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'disconnected',
    owner_id        UUID NOT NULL,
    last_sync_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_connections_owner ON connections(owner_id);
CREATE INDEX idx_connections_type ON connections(connector_type);

CREATE TABLE IF NOT EXISTS sync_jobs (
    id              UUID PRIMARY KEY,
    connection_id   UUID NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
    target_dataset_id UUID,
    table_name      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    rows_synced     BIGINT NOT NULL DEFAULT 0,
    error           TEXT,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_jobs_connection ON sync_jobs(connection_id);
