ALTER TABLE action_types
    ADD COLUMN IF NOT EXISTS authorization_policy JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS action_what_if_branches (
    id UUID PRIMARY KEY,
    action_id UUID NOT NULL REFERENCES action_types(id) ON DELETE CASCADE,
    target_object_id UUID REFERENCES object_instances(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    preview JSONB NOT NULL DEFAULT '{}'::jsonb,
    before_object JSONB,
    after_object JSONB,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_action_types_authorization_policy
    ON action_types USING GIN (authorization_policy);

CREATE INDEX IF NOT EXISTS idx_action_what_if_branches_action
    ON action_what_if_branches(action_id);

CREATE INDEX IF NOT EXISTS idx_action_what_if_branches_target_object
    ON action_what_if_branches(target_object_id);

CREATE INDEX IF NOT EXISTS idx_action_what_if_branches_owner
    ON action_what_if_branches(owner_id);

CREATE INDEX IF NOT EXISTS idx_action_what_if_branches_created_at
    ON action_what_if_branches(created_at DESC);
