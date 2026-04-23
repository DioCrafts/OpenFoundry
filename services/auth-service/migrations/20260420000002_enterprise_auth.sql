ALTER TABLE users
    ADD COLUMN IF NOT EXISTS organization_id UUID,
    ADD COLUMN IF NOT EXISTS attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS mfa_enforced BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS auth_source TEXT NOT NULL DEFAULT 'local';

ALTER TABLE permissions
    ADD COLUMN IF NOT EXISTS description TEXT;

CREATE TABLE IF NOT EXISTS group_roles (
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    role_id  UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, role_id)
);

CREATE TABLE IF NOT EXISTS abac_policies (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    effect      TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    resource    TEXT NOT NULL,
    action      TEXT NOT NULL,
    conditions  JSONB NOT NULL DEFAULT '{}'::jsonb,
    row_filter  TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_mfa_totp (
    user_id              UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    secret               TEXT NOT NULL,
    recovery_code_hashes JSONB NOT NULL DEFAULT '[]'::jsonb,
    enabled              BOOLEAN NOT NULL DEFAULT false,
    verified_at          TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    prefix       TEXT NOT NULL UNIQUE,
    scopes       JSONB NOT NULL DEFAULT '[]'::jsonb,
    expires_at   TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys (user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_revoked ON api_keys (revoked_at);

CREATE TABLE IF NOT EXISTS sso_providers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug              TEXT NOT NULL UNIQUE,
    name              TEXT NOT NULL,
    provider_type     TEXT NOT NULL CHECK (provider_type IN ('oidc', 'saml')),
    enabled           BOOLEAN NOT NULL DEFAULT true,
    client_id         TEXT,
    client_secret     TEXT,
    issuer_url        TEXT,
    authorization_url TEXT,
    token_url         TEXT,
    userinfo_url      TEXT,
    scopes            TEXT[] NOT NULL DEFAULT ARRAY['openid', 'profile', 'email']::TEXT[],
    saml_metadata_url TEXT,
    saml_entity_id    TEXT,
    saml_sso_url      TEXT,
    saml_certificate  TEXT,
    attribute_mapping JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS external_identities (
    provider_id UUID NOT NULL REFERENCES sso_providers(id) ON DELETE CASCADE,
    subject     TEXT NOT NULL,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email       TEXT,
    raw_claims  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (provider_id, subject)
);

CREATE INDEX IF NOT EXISTS idx_external_identities_user ON external_identities (user_id);

INSERT INTO permissions (resource, action, description) VALUES
    ('users', 'read', 'View users and membership'),
    ('users', 'write', 'Create, update, or deactivate users'),
    ('roles', 'read', 'View roles'),
    ('roles', 'write', 'Create roles and assign memberships'),
    ('groups', 'read', 'View groups'),
    ('groups', 'write', 'Create groups and assign roles'),
    ('permissions', 'read', 'View permission catalog'),
    ('permissions', 'write', 'Manage permission assignments'),
    ('policies', 'read', 'View ABAC policies'),
    ('policies', 'write', 'Manage ABAC policies'),
    ('api_keys', 'self', 'Manage own API keys'),
    ('api_keys', 'write', 'Manage any API key'),
    ('mfa', 'self', 'Manage own MFA settings'),
    ('sso', 'read', 'View SSO providers'),
    ('sso', 'write', 'Manage SSO providers')
ON CONFLICT (resource, action) DO UPDATE SET description = EXCLUDED.description;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('users', 'read'),
    ('roles', 'read'),
    ('groups', 'read'),
    ('permissions', 'read'),
    ('policies', 'read'),
    ('api_keys', 'self'),
    ('mfa', 'self'),
    ('sso', 'read')
)
WHERE r.name = 'editor'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('api_keys', 'self'),
    ('mfa', 'self')
)
WHERE r.name = 'viewer'
ON CONFLICT DO NOTHING;