CREATE TABLE IF NOT EXISTS notepad_documents (
    id              UUID PRIMARY KEY,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    owner_id        UUID NOT NULL,
    content         TEXT NOT NULL DEFAULT '',
    template_key    TEXT,
    widgets         JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_indexed_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notepad_documents_updated
    ON notepad_documents(updated_at DESC, created_at DESC);

CREATE TABLE IF NOT EXISTS notepad_presence (
    id            UUID PRIMARY KEY,
    document_id   UUID NOT NULL REFERENCES notepad_documents(id) ON DELETE CASCADE,
    user_id       UUID NOT NULL,
    session_id    TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    cursor_label  TEXT NOT NULL DEFAULT '',
    color         TEXT NOT NULL DEFAULT '#0f766e',
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_notepad_presence_unique_session
    ON notepad_presence(document_id, user_id, session_id);

CREATE INDEX IF NOT EXISTS idx_notepad_presence_last_seen
    ON notepad_presence(document_id, last_seen_at DESC);
