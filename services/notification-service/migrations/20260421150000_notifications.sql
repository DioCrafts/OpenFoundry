CREATE TABLE IF NOT EXISTS notifications (
    id          UUID PRIMARY KEY,
    user_id     UUID,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'system',
    severity    TEXT NOT NULL DEFAULT 'info',
    status      TEXT NOT NULL DEFAULT 'unread',
    channels    JSONB NOT NULL DEFAULT '["in_app"]',
    metadata    JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_created ON notifications(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_user_status ON notifications(user_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS notification_deliveries (
    id              UUID PRIMARY KEY,
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    channel         TEXT NOT NULL,
    status          TEXT NOT NULL,
    response        TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notification_deliveries_notification ON notification_deliveries(notification_id, created_at DESC);

CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id             UUID PRIMARY KEY,
    in_app_enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    email_enabled       BOOLEAN NOT NULL DEFAULT FALSE,
    email_address       TEXT,
    slack_webhook_url   TEXT,
    teams_webhook_url   TEXT,
    digest_frequency    TEXT NOT NULL DEFAULT 'instant',
    quiet_hours         JSONB NOT NULL DEFAULT '{}',
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);