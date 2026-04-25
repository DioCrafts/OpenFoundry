ALTER TABLE marketplace_package_versions
    ADD COLUMN IF NOT EXISTS release_channel TEXT NOT NULL DEFAULT 'stable',
    ADD COLUMN IF NOT EXISTS packaged_resources JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE marketplace_installs
    ADD COLUMN IF NOT EXISTS release_channel TEXT NOT NULL DEFAULT 'stable',
    ADD COLUMN IF NOT EXISTS fleet_id UUID,
    ADD COLUMN IF NOT EXISTS maintenance_window JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS auto_upgrade_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS enrollment_branch TEXT;

CREATE TABLE IF NOT EXISTS marketplace_product_fleets (
    id UUID PRIMARY KEY,
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    environment TEXT NOT NULL,
    workspace_targets JSONB NOT NULL DEFAULT '[]'::jsonb,
    release_channel TEXT NOT NULL DEFAULT 'stable',
    auto_upgrade_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    maintenance_window JSONB NOT NULL DEFAULT jsonb_build_object(
        'timezone', 'UTC',
        'days', jsonb_build_array('sun'),
        'start_hour_utc', 2,
        'duration_minutes', 120
    ),
    branch_strategy TEXT NOT NULL DEFAULT 'isolated_branch_per_feature',
    rollout_strategy TEXT NOT NULL DEFAULT 'rolling',
    status TEXT NOT NULL DEFAULT 'active',
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS marketplace_enrollment_branches (
    id UUID PRIMARY KEY,
    fleet_id UUID NOT NULL REFERENCES marketplace_product_fleets(id) ON DELETE CASCADE,
    listing_id UUID NOT NULL REFERENCES marketplace_listings(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    repository_branch TEXT NOT NULL,
    source_release_channel TEXT NOT NULL DEFAULT 'stable',
    source_version TEXT,
    workspace_targets JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (fleet_id, name)
);

ALTER TABLE marketplace_installs
    DROP CONSTRAINT IF EXISTS marketplace_installs_fleet_id_fkey;

ALTER TABLE marketplace_installs
    ADD CONSTRAINT marketplace_installs_fleet_id_fkey
        FOREIGN KEY (fleet_id) REFERENCES marketplace_product_fleets(id) ON DELETE SET NULL;

UPDATE marketplace_package_versions
SET release_channel = CASE
    WHEN version LIKE '0.%' THEN 'beta'
    ELSE 'stable'
END
WHERE release_channel = 'stable';

UPDATE marketplace_package_versions
SET packaged_resources = CASE listing_id
    WHEN '0196839d-d210-7f8c-8a1d-7ab001040001' THEN jsonb_build_array(
        jsonb_build_object('kind', 'connector', 'name', 'Incident webhook adapter', 'resource_ref', 'connectors/live-ops/webhook', 'required', true),
        jsonb_build_object('kind', 'pipeline', 'name', 'Ops ingest pipeline', 'resource_ref', 'pipelines/live-ops-ingest', 'required', true)
    )
    WHEN '0196839d-d210-7f8c-8a1d-7ab001040002' THEN jsonb_build_array(
        jsonb_build_object('kind', 'widget', 'name', 'Geo Insight Widget', 'resource_ref', 'widgets/geo-insight', 'required', true),
        jsonb_build_object('kind', 'dashboard', 'name', 'Geo Ops Dashboard', 'resource_ref', 'dashboards/geo-ops', 'required', false)
    )
    WHEN '0196839d-d210-7f8c-8a1d-7ab001040003' THEN jsonb_build_array(
        jsonb_build_object('kind', 'agent', 'name', 'Workflow Copilot', 'resource_ref', 'agents/workflow-copilot', 'required', true),
        jsonb_build_object('kind', 'workflow', 'name', 'Eval rollout workflow', 'resource_ref', 'workflows/agent-evals', 'required', true)
    )
    WHEN '01968b70-4f20-71b8-a6f0-0f4000001001' THEN jsonb_build_array(
        jsonb_build_object('kind', 'app', 'name', 'Operations Center', 'resource_ref', 'apps/ops-center', 'required', true),
        jsonb_build_object('kind', 'ontology', 'name', 'Command ontology pack', 'resource_ref', 'ontology/operations-center', 'required', true)
    )
    ELSE packaged_resources
END
WHERE packaged_resources = '[]'::jsonb;

INSERT INTO marketplace_product_fleets (
    id,
    listing_id,
    name,
    environment,
    workspace_targets,
    release_channel,
    auto_upgrade_enabled,
    maintenance_window,
    branch_strategy,
    rollout_strategy,
    status,
    last_synced_at,
    created_at,
    updated_at
)
VALUES (
    '01968c10-2f20-71b8-a6f0-0f4000002001',
    '01968b70-4f20-71b8-a6f0-0f4000001001',
    'Ops Center Fleet',
    'production',
    jsonb_build_array('Operations Center - EU', 'Operations Center - US'),
    'stable',
    TRUE,
    jsonb_build_object(
        'timezone', 'UTC',
        'days', jsonb_build_array('sun'),
        'start_hour_utc', 2,
        'duration_minutes', 180
    ),
    'isolated_branch_per_feature',
    'rolling',
    'active',
    NOW() - interval '6 hours',
    NOW() - interval '4 days',
    NOW() - interval '1 hour'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO marketplace_enrollment_branches (
    id,
    fleet_id,
    listing_id,
    name,
    repository_branch,
    source_release_channel,
    source_version,
    workspace_targets,
    status,
    notes,
    created_at,
    updated_at
)
VALUES (
    '01968c10-2f20-71b8-a6f0-0f4000002101',
    '01968c10-2f20-71b8-a6f0-0f4000002001',
    '01968b70-4f20-71b8-a6f0-0f4000001001',
    'feature/shift-handovers',
    'release/ops-center/feature-shift-handovers',
    'stable',
    '1.0.0',
    jsonb_build_array('Operations Center - EU', 'Operations Center - US'),
    'active',
    'Enrollment branch for incident handover widgets and playbooks.',
    NOW() - interval '2 days',
    NOW() - interval '2 hours'
)
ON CONFLICT (fleet_id, name) DO NOTHING;

UPDATE marketplace_installs
SET release_channel = 'stable',
    fleet_id = '01968c10-2f20-71b8-a6f0-0f4000002001',
    maintenance_window = jsonb_build_object(
        'timezone', 'UTC',
        'days', jsonb_build_array('sun'),
        'start_hour_utc', 2,
        'duration_minutes', 180
    ),
    auto_upgrade_enabled = TRUE
WHERE id = '0196839d-d210-7f8c-8a1d-7ab001040301';
