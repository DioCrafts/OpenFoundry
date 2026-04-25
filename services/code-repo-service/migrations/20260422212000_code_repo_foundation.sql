CREATE TABLE IF NOT EXISTS code_repositories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    owner TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    visibility TEXT NOT NULL,
    object_store_backend TEXT NOT NULL,
    package_kind TEXT NOT NULL,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS code_repository_branches (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    head_sha TEXT NOT NULL,
    base_branch TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    protected BOOLEAN NOT NULL DEFAULT FALSE,
    ahead_by INTEGER NOT NULL DEFAULT 0,
    pending_reviews INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS code_repository_commits (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL,
    sha TEXT NOT NULL,
    parent_sha TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    author_name TEXT NOT NULL,
    author_email TEXT NOT NULL,
    files_changed INTEGER NOT NULL DEFAULT 0,
    additions INTEGER NOT NULL DEFAULT 0,
    deletions INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS code_repository_files (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    language TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL DEFAULT '',
    last_commit_sha TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS code_merge_requests (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_branch TEXT NOT NULL,
    target_branch TEXT NOT NULL,
    status TEXT NOT NULL,
    author TEXT NOT NULL,
    labels JSONB NOT NULL DEFAULT '[]'::jsonb,
    reviewers JSONB NOT NULL DEFAULT '[]'::jsonb,
    approvals_required INTEGER NOT NULL DEFAULT 2,
    changed_files INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    merged_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS code_review_comments (
    id UUID PRIMARY KEY,
    merge_request_id UUID NOT NULL REFERENCES code_merge_requests(id) ON DELETE CASCADE,
    author TEXT NOT NULL,
    body TEXT NOT NULL,
    file_path TEXT NOT NULL DEFAULT '',
    line_number INTEGER,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS code_ci_runs (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES code_repositories(id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    pipeline_name TEXT NOT NULL,
    status TEXT NOT NULL,
    trigger TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    checks JSONB NOT NULL DEFAULT '[]'::jsonb
);

INSERT INTO code_repositories (id, name, slug, description, owner, default_branch, visibility, object_store_backend, package_kind, tags, settings, created_at, updated_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001030001',
    'Foundry Widget Kit',
    'foundry-widget-kit',
    'Shared widget primitives ready to publish into the internal marketplace.',
    'Platform UI',
    'main',
    'private',
    'gitoxide-pack',
    'widget',
    jsonb_build_array('widgets', 'ui', 'marketplace'),
    jsonb_build_object('default_path', 'src/lib.rs', 'ci_required', true),
    NOW() - interval '24 days',
    NOW() - interval '4 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001030002',
    'Ops Connector Pack',
    'ops-connector-pack',
    'Connector templates for webhook and message bus ingestion.',
    'Data Platform',
    'main',
    'private',
    'gitoxide-pack',
    'connector',
    jsonb_build_array('connectors', 'ops'),
    jsonb_build_object('default_path', 'src/connectors.rs', 'ci_required', true),
    NOW() - interval '18 days',
    NOW() - interval '6 hours'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_repository_branches (id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001030101', '0196839d-d210-7f8c-8a1d-7ab001030001', 'main', 'a12b3c4', NULL, true, true, 0, 0, NOW() - interval '4 hours'),
('0196839d-d210-7f8c-8a1d-7ab001030102', '0196839d-d210-7f8c-8a1d-7ab001030001', 'feature/map-preview', 'b34c5d6', 'main', false, false, 2, 1, NOW() - interval '90 minutes'),
('0196839d-d210-7f8c-8a1d-7ab001030103', '0196839d-d210-7f8c-8a1d-7ab001030002', 'main', 'c45d6e7', NULL, true, true, 0, 0, NOW() - interval '6 hours')
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_repository_commits (id, repository_id, branch_name, sha, parent_sha, title, description, author_name, author_email, files_changed, additions, deletions, created_at)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001030201', '0196839d-d210-7f8c-8a1d-7ab001030001', 'feature/map-preview', 'b34c5d6', 'a12b3c4', 'Add map preview widget shell', 'Introduces preview panel and example widget metadata.', 'Nadia Reyes', 'nadia.reyes@openfoundry.dev', 4, 96, 12, NOW() - interval '90 minutes'),
('0196839d-d210-7f8c-8a1d-7ab001030202', '0196839d-d210-7f8c-8a1d-7ab001030001', 'main', 'a12b3c4', NULL, 'Seed widget library', 'Initial repository scaffold for widget publishing.', 'Nadia Reyes', 'nadia.reyes@openfoundry.dev', 3, 52, 0, NOW() - interval '4 hours'),
('0196839d-d210-7f8c-8a1d-7ab001030203', '0196839d-d210-7f8c-8a1d-7ab001030002', 'main', 'c45d6e7', NULL, 'Create connector package template', 'Adds connector scaffold and package metadata.', 'Amir Solis', 'amir.solis@openfoundry.dev', 5, 81, 0, NOW() - interval '6 hours')
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_repository_files (id, repository_id, path, branch_name, language, size_bytes, content, last_commit_sha)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001030301', '0196839d-d210-7f8c-8a1d-7ab001030001', 'README.md', 'main', 'markdown', 1640, '# Foundry Widget Kit\n\nShared widget primitives for marketplace publication.\n', 'a12b3c4'),
('0196839d-d210-7f8c-8a1d-7ab001030302', '0196839d-d210-7f8c-8a1d-7ab001030001', 'src/lib.rs', 'main', 'rust', 840, $$pub fn widget_entry() -> &'static str {\n    "widget-kit"\n}\n$$, 'a12b3c4'),
('0196839d-d210-7f8c-8a1d-7ab001030303', '0196839d-d210-7f8c-8a1d-7ab001030001', 'src/map_preview.rs', 'feature/map-preview', 'rust', 1120, $$pub fn render_preview() -> &'static str {\n    "map-preview"\n}\n$$, 'b34c5d6'),
('0196839d-d210-7f8c-8a1d-7ab001030304', '0196839d-d210-7f8c-8a1d-7ab001030002', 'src/connectors.rs', 'main', 'rust', 980, $$pub fn webhook_connector() -> &'static str {\n    "webhook"\n}\n$$, 'c45d6e7')
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_merge_requests (id, repository_id, title, description, source_branch, target_branch, status, author, labels, reviewers, approvals_required, changed_files, created_at, updated_at, merged_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001030401',
    '0196839d-d210-7f8c-8a1d-7ab001030001',
    'Add map preview widget shell',
    'Creates the first widget shell used by the marketplace preview experience.',
    'feature/map-preview',
    'main',
    'open',
    'Nadia Reyes',
    jsonb_build_array('widget', 'preview'),
    jsonb_build_array(
        jsonb_build_object('reviewer', 'Marco', 'approved', true, 'state', 'approved'),
        jsonb_build_object('reviewer', 'Elena', 'approved', false, 'state', 'changes_requested')
    ),
    2,
    4,
    NOW() - interval '70 minutes',
    NOW() - interval '20 minutes',
    NULL
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_review_comments (id, merge_request_id, author, body, file_path, line_number, resolved, created_at)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001030501', '0196839d-d210-7f8c-8a1d-7ab001030401', 'Elena', 'Please split the preview renderer into a separate module before merge.', 'src/map_preview.rs', 12, false, NOW() - interval '18 minutes'),
('0196839d-d210-7f8c-8a1d-7ab001030502', '0196839d-d210-7f8c-8a1d-7ab001030401', 'Marco', 'Naming looks good; CI is green.', 'src/map_preview.rs', NULL, true, NOW() - interval '16 minutes')
ON CONFLICT (id) DO NOTHING;

INSERT INTO code_ci_runs (id, repository_id, branch_name, commit_sha, pipeline_name, status, trigger, started_at, completed_at, checks)
VALUES
('0196839d-d210-7f8c-8a1d-7ab001030601', '0196839d-d210-7f8c-8a1d-7ab001030001', 'feature/map-preview', 'b34c5d6', 'package-validation', 'passed', 'push', NOW() - interval '60 minutes', NOW() - interval '56 minutes', jsonb_build_array('cargo check', 'package lint', 'widget smoke test')),
('0196839d-d210-7f8c-8a1d-7ab001030602', '0196839d-d210-7f8c-8a1d-7ab001030002', 'main', 'c45d6e7', 'package-validation', 'passed', 'push', NOW() - interval '5 hours', NOW() - interval '4 hours 55 minutes', jsonb_build_array('cargo check', 'connector integration', 'publish dry run'))
ON CONFLICT (id) DO NOTHING;
