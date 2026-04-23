CREATE TABLE IF NOT EXISTS workflows (
    id               UUID PRIMARY KEY,
    name             TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    owner_id         UUID NOT NULL,
    status           TEXT NOT NULL DEFAULT 'draft',
    trigger_type     TEXT NOT NULL,
    trigger_config   JSONB NOT NULL DEFAULT '{}',
    steps            JSONB NOT NULL DEFAULT '[]',
    webhook_secret   TEXT,
    next_run_at      TIMESTAMPTZ,
    last_triggered_at TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workflows_owner ON workflows(owner_id);
CREATE INDEX IF NOT EXISTS idx_workflows_trigger ON workflows(trigger_type, status, next_run_at);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id              UUID PRIMARY KEY,
    workflow_id     UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    trigger_type    TEXT NOT NULL,
    status          TEXT NOT NULL,
    started_by      UUID,
    current_step_id TEXT,
    context         JSONB NOT NULL DEFAULT '{}',
    error_message   TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow ON workflow_runs(workflow_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_status ON workflow_runs(status);

CREATE TABLE IF NOT EXISTS workflow_approvals (
    id             UUID PRIMARY KEY,
    workflow_id    UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_run_id UUID NOT NULL REFERENCES workflow_runs(id) ON DELETE CASCADE,
    step_id        TEXT NOT NULL,
    title          TEXT NOT NULL,
    instructions   TEXT NOT NULL DEFAULT '',
    assigned_to    UUID,
    status         TEXT NOT NULL DEFAULT 'pending',
    decision       TEXT,
    payload        JSONB NOT NULL DEFAULT '{}',
    requested_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    decided_at     TIMESTAMPTZ,
    decided_by     UUID
);

CREATE INDEX IF NOT EXISTS idx_workflow_approvals_assigned ON workflow_approvals(assigned_to, status, requested_at DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_approvals_run ON workflow_approvals(workflow_run_id);