CREATE TABLE IF NOT EXISTS marketplace_listings (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    summary TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    publisher TEXT NOT NULL,
    category_slug TEXT NOT NULL,
    package_kind TEXT NOT NULL,
    repository_slug TEXT NOT NULL,
    visibility TEXT NOT NULL,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
    install_count BIGINT NOT NULL DEFAULT 0,
    average_rating DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS marketplace_package_versions (
    id UUID PRIMARY KEY,
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    changelog TEXT NOT NULL,
    dependency_mode TEXT NOT NULL,
    dependencies JSONB NOT NULL DEFAULT '[]'::jsonb,
    manifest JSONB NOT NULL DEFAULT '{}'::jsonb,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS marketplace_reviews (
    id UUID PRIMARY KEY,
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    author TEXT NOT NULL,
    rating INTEGER NOT NULL,
    headline TEXT NOT NULL,
    body TEXT NOT NULL,
    recommended BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS marketplace_installs (
    id UUID PRIMARY KEY,
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    listing_name TEXT NOT NULL,
    version TEXT NOT NULL,
    workspace_name TEXT NOT NULL,
    status TEXT NOT NULL,
    dependency_plan JSONB NOT NULL DEFAULT '[]'::jsonb,
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ready_at TIMESTAMPTZ
);

INSERT INTO marketplace_listings (id, name, slug, summary, description, publisher, category_slug, package_kind, repository_slug, visibility, tags, capabilities, install_count, average_rating, created_at, updated_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001040001',
    'Live Ops Connector',
    'live-ops-connector',
    'Connector pack for streaming operational incidents into OpenFoundry.',
    'Ships webhook and Kafka ingestion adapters with monitoring hooks.',
    'Data Platform',
    'connectors',
    'connector',
    'ops-connector-pack',
    'private',
    jsonb_build_array('streaming', 'ops', 'alerts'),
    jsonb_build_array('webhook', 'kafka', 'retry-policies'),
    41,
    4.7,
    NOW() - interval '30 days',
    NOW() - interval '3 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001040002',
    'Geo Insight Widget',
    'geo-insight-widget',
    'Map widget with clustering and route overlays for dashboards.',
    'Provides a marketplace-ready geospatial widget powered by MapLibre previews.',
    'Platform UI',
    'widgets',
    'widget',
    'foundry-widget-kit',
    'private',
    jsonb_build_array('maps', 'dashboard', 'geospatial'),
    jsonb_build_array('maplibre', 'clusters', 'routes'),
    26,
    4.9,
    NOW() - interval '16 days',
    NOW() - interval '45 minutes'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001040003',
    'Agent Workflow Starter',
    'agent-workflow-starter',
    'Template repo for internal AI-agent workflows and copilots.',
    'Bundles orchestrator setup, prompt contracts, and deployment wiring.',
    'AI Enablement',
    'ai-agents',
    'ai_agent',
    'agent-workflow-starter',
    'private',
    jsonb_build_array('agents', 'starter', 'workflow'),
    jsonb_build_array('prompt-packs', 'evals', 'deployment'),
    18,
    4.5,
    NOW() - interval '10 days',
    NOW() - interval '2 hours'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO marketplace_package_versions (id, listing_id, version, changelog, dependency_mode, dependencies, manifest, published_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001040101',
    '0196839d-d210-7f8c-8a1d-7ab001040001',
    '1.4.0',
    'Adds Slack retry routing and hardened webhook signatures.',
    'strict',
    jsonb_build_array(
        jsonb_build_object('package_slug', 'ops-runtime', 'version_req', '^2.3', 'required', true)
    ),
    jsonb_build_object('entrypoint', 'connector.yaml', 'runtime', 'rust'),
    NOW() - interval '3 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001040102',
    '0196839d-d210-7f8c-8a1d-7ab001040002',
    '0.9.2',
    'Introduces route overlay presets and heatmap defaults.',
    'strict',
    jsonb_build_array(
        jsonb_build_object('package_slug', 'map-style-base', 'version_req', '~1.1', 'required', true)
    ),
    jsonb_build_object('entrypoint', 'widget.json', 'runtime', 'svelte'),
    NOW() - interval '45 minutes'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001040103',
    '0196839d-d210-7f8c-8a1d-7ab001040003',
    '0.3.0',
    'Packages evaluation harness and deployment scaffolding.',
    'strict',
    jsonb_build_array(
        jsonb_build_object('package_slug', 'agent-evals', 'version_req', '^0.8', 'required', true),
        jsonb_build_object('package_slug', 'workflow-runtime', 'version_req', '^1.2', 'required', true)
    ),
    jsonb_build_object('entrypoint', 'agent.toml', 'runtime', 'python'),
    NOW() - interval '2 hours'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO marketplace_reviews (id, listing_id, author, rating, headline, body, recommended, created_at)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001040201', '0196839d-d210-7f8c-8a1d-7ab001040001', 'Sofia', 5, 'Fast connector rollout', 'We installed this connector pack into staging in under ten minutes.', true, NOW() - interval '2 days'),
('0196839d-d210-7f8c-8a1d-7ab001040202', '0196839d-d210-7f8c-8a1d-7ab001040002', 'Marco', 5, 'Great dashboard widget', 'The cluster and route presets made the map demo usable on day one.', true, NOW() - interval '1 day'),
('0196839d-d210-7f8c-8a1d-7ab001040203', '0196839d-d210-7f8c-8a1d-7ab001040003', 'Lucia', 4, 'Solid agent starter', 'We still customized the deployment target, but the workflow skeleton was solid.', true, NOW() - interval '12 hours')
ON CONFLICT (id) DO NOTHING;

INSERT INTO marketplace_installs (id, listing_id, listing_name, version, workspace_name, status, dependency_plan, installed_at, ready_at)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001040301', '0196839d-d210-7f8c-8a1d-7ab001040002', 'Geo Insight Widget', '0.9.2', 'Geo Analytics Workspace', 'installed', jsonb_build_array(jsonb_build_object('package_slug', 'map-style-base', 'version_req', '~1.1', 'required', true)), NOW() - interval '25 minutes', NOW() - interval '23 minutes')
ON CONFLICT (id) DO NOTHING;