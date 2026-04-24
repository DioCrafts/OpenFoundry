CREATE TABLE IF NOT EXISTS streaming_events (
    id UUID PRIMARY KEY,
    stream_id UUID NOT NULL REFERENCES streaming_streams(id) ON DELETE CASCADE,
    sequence_no BIGINT GENERATED ALWAYS AS IDENTITY,
    payload JSONB NOT NULL,
    event_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    archive_path TEXT
);

CREATE INDEX IF NOT EXISTS idx_streaming_events_stream_sequence
    ON streaming_events(stream_id, sequence_no);

CREATE INDEX IF NOT EXISTS idx_streaming_events_stream_archived
    ON streaming_events(stream_id, archived_at, event_time);

CREATE TABLE IF NOT EXISTS streaming_checkpoints (
    topology_id UUID NOT NULL REFERENCES streaming_topologies(id) ON DELETE CASCADE,
    stream_id UUID NOT NULL REFERENCES streaming_streams(id) ON DELETE CASCADE,
    last_event_id UUID,
    last_sequence_no BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (topology_id, stream_id)
);

CREATE TABLE IF NOT EXISTS streaming_lineage_edges (
    id UUID PRIMARY KEY,
    source_stream_id UUID NOT NULL REFERENCES streaming_streams(id) ON DELETE CASCADE,
    target_dataset_id UUID NOT NULL,
    topology_id UUID NOT NULL REFERENCES streaming_topologies(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (source_stream_id, target_dataset_id, topology_id)
);

CREATE INDEX IF NOT EXISTS idx_streaming_lineage_topology
    ON streaming_lineage_edges(topology_id);
