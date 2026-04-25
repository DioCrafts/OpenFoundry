CREATE TABLE IF NOT EXISTS streaming_dead_letters (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES streaming_streams(id) ON DELETE CASCADE,
    payload JSONB NOT NULL,
    event_time TIMESTAMPTZ NOT NULL,
    reason TEXT NOT NULL,
    validation_errors JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'queued',
    replay_count INTEGER NOT NULL DEFAULT 0,
    last_replayed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT streaming_dead_letters_status_valid
        CHECK (status IN ('queued', 'replayed', 'ignored'))
);

CREATE INDEX IF NOT EXISTS idx_streaming_dead_letters_stream_status
    ON streaming_dead_letters(stream_id, status, created_at DESC);
