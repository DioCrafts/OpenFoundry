CREATE TABLE IF NOT EXISTS dataset_profiles (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL UNIQUE REFERENCES datasets(id) ON DELETE CASCADE,
    profile     JSONB NOT NULL DEFAULT '{}',
    score       DOUBLE PRECISION NOT NULL DEFAULT 0,
    alerts      JSONB NOT NULL DEFAULT '[]',
    profiled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dataset_profiles_dataset ON dataset_profiles(dataset_id);

CREATE TABLE IF NOT EXISTS dataset_quality_rules (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    rule_type   TEXT NOT NULL,
    severity    TEXT NOT NULL DEFAULT 'medium',
    config      JSONB NOT NULL DEFAULT '{}',
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dataset_quality_rules_dataset ON dataset_quality_rules(dataset_id);

CREATE TABLE IF NOT EXISTS dataset_quality_history (
    id           UUID PRIMARY KEY,
    dataset_id   UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    score        DOUBLE PRECISION NOT NULL,
    passed_rules INTEGER NOT NULL DEFAULT 0,
    failed_rules INTEGER NOT NULL DEFAULT 0,
    alerts_count INTEGER NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dataset_quality_history_dataset_created
    ON dataset_quality_history(dataset_id, created_at DESC);

CREATE TABLE IF NOT EXISTS dataset_quality_alerts (
    id          UUID PRIMARY KEY,
    dataset_id  UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    level       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    message     TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    details     JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_dataset_quality_alerts_dataset_status
    ON dataset_quality_alerts(dataset_id, status, created_at DESC);