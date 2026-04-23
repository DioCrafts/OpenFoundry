-- Query service schema

CREATE TABLE IF NOT EXISTS saved_queries (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    sql         TEXT NOT NULL,
    owner_id    UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_saved_queries_owner ON saved_queries(owner_id);
CREATE INDEX idx_saved_queries_name ON saved_queries(name);
