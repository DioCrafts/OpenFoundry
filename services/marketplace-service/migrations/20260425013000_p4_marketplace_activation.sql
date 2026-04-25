ALTER TABLE marketplace_installs
    ADD COLUMN IF NOT EXISTS activation JSONB NOT NULL DEFAULT '{}'::jsonb;

INSERT INTO marketplace_listings (id, name, slug, summary, description, publisher, category_slug, package_kind, repository_slug, visibility, tags, capabilities, install_count, average_rating, created_at, updated_at)
VALUES
(
    '01968b70-4f20-71b8-a6f0-0f4000001001',
    'Operations Center Template',
    'operations-center-template',
    'Published Workshop template for operational command views.',
    'Installs a real Workshop app from the seeded ops-center template and publishes it immediately.',
    'Platform Apps',
    'app-templates',
    'app_template',
    'ops-center-template',
    'private',
    jsonb_build_array('workshop', 'template', 'operations'),
    jsonb_build_array('published-runtime', 'dashboard', 'marketplace-install'),
    0,
    0,
    NOW(),
    NOW()
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO marketplace_package_versions (id, listing_id, version, changelog, dependency_mode, dependencies, manifest, published_at)
VALUES
(
    '01968b70-4f20-71b8-a6f0-0f4000001101',
    '01968b70-4f20-71b8-a6f0-0f4000001001',
    '1.0.0',
    'Initial app-template package wired to create and publish a Workshop app during install.',
    'strict',
    '[]'::jsonb,
    jsonb_build_object(
        'template_key', 'ops-center',
        'runtime', 'app_builder',
        'activation_mode', 'create_and_publish'
    ),
    NOW()
)
ON CONFLICT (id) DO NOTHING;
