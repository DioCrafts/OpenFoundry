CREATE TABLE IF NOT EXISTS ontology_function_package_runs (
    id UUID PRIMARY KEY,
    function_package_id UUID NOT NULL REFERENCES ontology_function_packages(id) ON DELETE CASCADE,
    function_package_name TEXT NOT NULL,
    function_package_version TEXT NOT NULL,
    runtime TEXT NOT NULL,
    status TEXT NOT NULL,
    invocation_kind TEXT NOT NULL,
    action_id UUID,
    action_name TEXT,
    object_type_id UUID,
    target_object_id UUID,
    actor_id UUID NOT NULL,
    duration_ms BIGINT NOT NULL CHECK (duration_ms >= 0),
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS ontology_function_package_runs_package_completed_idx
    ON ontology_function_package_runs (function_package_id, completed_at DESC);

CREATE INDEX IF NOT EXISTS ontology_function_package_runs_package_status_idx
    ON ontology_function_package_runs (function_package_id, status);

CREATE INDEX IF NOT EXISTS ontology_function_package_runs_package_invocation_idx
    ON ontology_function_package_runs (function_package_id, invocation_kind);
