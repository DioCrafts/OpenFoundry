CREATE TABLE IF NOT EXISTS ontology_projects (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    workspace_slug TEXT NULL,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ontology_project_memberships (
    project_id UUID NOT NULL REFERENCES ontology_projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'owner')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, user_id)
);

CREATE TABLE IF NOT EXISTS ontology_project_resources (
    project_id UUID NOT NULL REFERENCES ontology_projects(id) ON DELETE CASCADE,
    resource_kind TEXT NOT NULL,
    resource_id UUID NOT NULL,
    bound_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (resource_kind, resource_id)
);

CREATE INDEX IF NOT EXISTS idx_ontology_projects_owner
    ON ontology_projects(owner_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_ontology_projects_workspace
    ON ontology_projects(workspace_slug);

CREATE INDEX IF NOT EXISTS idx_ontology_project_memberships_user
    ON ontology_project_memberships(user_id, role);

CREATE INDEX IF NOT EXISTS idx_ontology_project_resources_project
    ON ontology_project_resources(project_id, resource_kind, created_at DESC);
