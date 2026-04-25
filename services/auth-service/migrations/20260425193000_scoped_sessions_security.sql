CREATE TABLE IF NOT EXISTS scoped_sessions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label        TEXT NOT NULL,
    session_kind TEXT NOT NULL CHECK (session_kind IN ('scoped', 'guest')),
    scope        JSONB NOT NULL DEFAULT '{}'::jsonb,
    guest_email  TEXT,
    guest_name   TEXT,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scoped_sessions_user ON scoped_sessions (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_scoped_sessions_active ON scoped_sessions (expires_at, revoked_at);

INSERT INTO permissions (resource, action, description) VALUES
    ('sessions', 'self', 'Issue and revoke own scoped sessions'),
    ('sessions', 'write', 'Manage scoped sessions for any user'),
    ('guests', 'write', 'Issue guest sessions and external sharing credentials')
ON CONFLICT (resource, action) DO UPDATE SET description = EXCLUDED.description;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('sessions', 'self'),
    ('sessions', 'write'),
    ('guests', 'write')
)
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('sessions', 'self')
)
WHERE r.name IN ('editor', 'viewer')
ON CONFLICT DO NOTHING;
