CREATE TABLE IF NOT EXISTS restricted_views (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT NOT NULL UNIQUE,
    description           TEXT,
    resource              TEXT NOT NULL,
    action                TEXT NOT NULL,
    conditions            JSONB NOT NULL DEFAULT '{}'::jsonb,
    row_filter            TEXT,
    hidden_columns        JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_org_ids       JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_markings      JSONB NOT NULL DEFAULT '[]'::jsonb,
    consumer_mode_enabled BOOLEAN NOT NULL DEFAULT false,
    allow_guest_access    BOOLEAN NOT NULL DEFAULT false,
    enabled               BOOLEAN NOT NULL DEFAULT true,
    created_by            UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_restricted_views_lookup
    ON restricted_views (resource, action, enabled, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_restricted_views_conditions
    ON restricted_views USING GIN (conditions);

INSERT INTO permissions (resource, action, description) VALUES
    ('restricted_views', 'read', 'View restricted views and consumer-mode data cuts'),
    ('restricted_views', 'write', 'Manage restricted views and consumer-mode data cuts')
ON CONFLICT (resource, action) DO UPDATE SET description = EXCLUDED.description;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('restricted_views', 'read'),
    ('restricted_views', 'write')
)
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('restricted_views', 'read')
)
WHERE r.name IN ('editor', 'viewer')
ON CONFLICT DO NOTHING;
