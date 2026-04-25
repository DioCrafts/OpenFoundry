CREATE TABLE IF NOT EXISTS control_panel_settings (
    singleton_id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton_id),
    platform_name TEXT NOT NULL DEFAULT 'OpenFoundry',
    support_email TEXT NOT NULL DEFAULT 'support@openfoundry.dev',
    docs_url TEXT NOT NULL DEFAULT 'https://docs.openfoundry.dev',
    status_page_url TEXT NOT NULL DEFAULT 'https://status.openfoundry.dev',
    announcement_banner TEXT NOT NULL DEFAULT '',
    maintenance_mode BOOLEAN NOT NULL DEFAULT FALSE,
    release_channel TEXT NOT NULL DEFAULT 'stable',
    default_region TEXT NOT NULL DEFAULT 'eu-west-1',
    deployment_mode TEXT NOT NULL DEFAULT 'self_hosted',
    allow_self_signup BOOLEAN NOT NULL DEFAULT FALSE,
    allowed_email_domains JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_app_branding JSONB NOT NULL DEFAULT jsonb_build_object(
        'display_name', 'OpenFoundry',
        'primary_color', '#0f766e',
        'accent_color', '#0f172a',
        'logo_url', NULL,
        'favicon_url', NULL,
        'show_powered_by', TRUE
    ),
    restricted_operations JSONB NOT NULL DEFAULT '["terraform-apply", "marketplace-publish-prod"]'::jsonb,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO control_panel_settings (singleton_id)
VALUES (TRUE)
ON CONFLICT (singleton_id) DO NOTHING;

INSERT INTO permissions (resource, action, description) VALUES
    ('control_panel', 'read', 'View platform control panel settings'),
    ('control_panel', 'write', 'Manage branding, release, and platform controls')
ON CONFLICT (resource, action) DO UPDATE SET description = EXCLUDED.description;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
INNER JOIN permissions p ON (p.resource, p.action) IN (
    ('control_panel', 'read'),
    ('control_panel', 'write')
)
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;
