CREATE TABLE IF NOT EXISTS code_repository_integrations (
	id UUID PRIMARY KEY,
	repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
	provider TEXT NOT NULL,
	external_namespace TEXT NOT NULL,
	external_project TEXT NOT NULL,
	external_url TEXT NOT NULL,
	sync_mode TEXT NOT NULL,
	ci_trigger_strategy TEXT NOT NULL,
	status TEXT NOT NULL,
	default_branch TEXT NOT NULL,
	branch_mapping JSONB NOT NULL DEFAULT '[]'::jsonb,
	webhook_url TEXT NOT NULL,
	last_synced_at TIMESTAMPTZ,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS code_repository_sync_runs (
	id UUID PRIMARY KEY,
	integration_id UUID NOT NULL REFERENCES code_repository_integrations(id) ON DELETE CASCADE,
	repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
	trigger TEXT NOT NULL,
	status TEXT NOT NULL,
	commit_sha TEXT NOT NULL,
	branch_name TEXT NOT NULL,
	summary TEXT NOT NULL,
	checks JSONB NOT NULL DEFAULT '[]'::jsonb,
	started_at TIMESTAMPTZ NOT NULL,
	completed_at TIMESTAMPTZ
);

INSERT INTO code_repository_integrations (
	id,
	repository_id,
	provider,
	external_namespace,
	external_project,
	external_url,
	sync_mode,
	ci_trigger_strategy,
	status,
	default_branch,
	branch_mapping,
	webhook_url,
	last_synced_at,
	created_at,
	updated_at
)
VALUES
(
	'01968840-36a1-7424-a991-1a0f10000101',
	'0196839d-d210-7f8c-8a1d-7ab001030001',
	'github',
	'open-foundry-labs',
	'foundry-widget-kit',
	'https://github.com/open-foundry-labs/foundry-widget-kit',
	'bidirectional_mirror',
	'github_actions',
	'connected',
	'main',
	jsonb_build_array('main->main', 'feature/*->feature/*'),
	'https://code-repo.openfoundry.local/hooks/github/foundry-widget-kit',
	NOW() - interval '52 minutes',
	NOW() - interval '8 days',
	NOW() - interval '52 minutes'
),
(
	'01968840-36a1-7424-a991-1a0f10000102',
	'0196839d-d210-7f8c-8a1d-7ab001030002',
	'gitlab',
	'open-foundry',
	'ops-connector-pack',
	'https://gitlab.com/open-foundry/ops-connector-pack',
	'push_mirror',
	'gitlab_ci',
	'connected',
	'main',
	jsonb_build_array('main->main'),
	'https://code-repo.openfoundry.local/hooks/gitlab/ops-connector-pack',
	NOW() - interval '4 hours 42 minutes',
	NOW() - interval '5 days',
	NOW() - interval '4 hours 42 minutes'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_repository_sync_runs (
	id,
	integration_id,
	repository_id,
	trigger,
	status,
	commit_sha,
	branch_name,
	summary,
	checks,
	started_at,
	completed_at
)
VALUES
(
	'01968840-36a1-7424-a991-1a0f10000201',
	'01968840-36a1-7424-a991-1a0f10000101',
	'0196839d-d210-7f8c-8a1d-7ab001030001',
	'webhook',
	'completed',
	'b34c5d6',
	'feature/map-preview',
	'GitHub sync completed and GitHub Actions pipeline triggered for feature/map-preview.',
	jsonb_build_array('fetch remote', 'mirror refs', 'dispatch github_actions'),
	NOW() - interval '52 minutes',
	NOW() - interval '48 minutes'
),
(
	'01968840-36a1-7424-a991-1a0f10000202',
	'01968840-36a1-7424-a991-1a0f10000102',
	'0196839d-d210-7f8c-8a1d-7ab001030002',
	'push',
	'completed',
	'c45d6e7',
	'main',
	'GitLab push mirror completed and downstream pipeline validated connector package publish path.',
	jsonb_build_array('push mirror', 'dispatch gitlab_ci', 'publish dry run'),
	NOW() - interval '4 hours 42 minutes',
	NOW() - interval '4 hours 37 minutes'
)
ON CONFLICT (id) DO NOTHING;