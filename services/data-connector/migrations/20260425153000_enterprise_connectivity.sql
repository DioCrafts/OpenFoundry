CREATE TABLE IF NOT EXISTS connector_agents (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    agent_url TEXT NOT NULL,
    owner_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'online',
    capabilities JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    last_heartbeat_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_connector_agents_owner ON connector_agents(owner_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_connector_agents_agent_url ON connector_agents(agent_url);

CREATE TABLE IF NOT EXISTS connection_registrations (
    id UUID PRIMARY KEY,
    connection_id UUID NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
    selector TEXT NOT NULL,
    display_name TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    registration_mode TEXT NOT NULL DEFAULT 'sync',
    auto_sync BOOLEAN NOT NULL DEFAULT FALSE,
    update_detection BOOLEAN NOT NULL DEFAULT TRUE,
    target_dataset_id UUID,
    last_source_signature TEXT,
    last_dataset_version INT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_connection_registrations_unique
    ON connection_registrations(connection_id, selector);
CREATE INDEX IF NOT EXISTS idx_connection_registrations_connection
    ON connection_registrations(connection_id, created_at DESC);
