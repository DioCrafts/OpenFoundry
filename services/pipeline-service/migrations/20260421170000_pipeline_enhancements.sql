ALTER TABLE pipelines
    ADD COLUMN IF NOT EXISTS schedule_config JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS retry_policy JSONB NOT NULL DEFAULT '{"max_attempts": 1, "retry_on_failure": false, "allow_partial_reexecution": true}',
    ADD COLUMN IF NOT EXISTS next_run_at TIMESTAMPTZ;

ALTER TABLE pipeline_runs
    ADD COLUMN IF NOT EXISTS trigger_type TEXT NOT NULL DEFAULT 'manual',
    ADD COLUMN IF NOT EXISTS attempt_number INT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS started_from_node_id TEXT,
    ADD COLUMN IF NOT EXISTS retry_of_run_id UUID,
    ADD COLUMN IF NOT EXISTS execution_context JSONB NOT NULL DEFAULT '{}';

ALTER TABLE pipeline_runs
    ALTER COLUMN started_by DROP NOT NULL;

CREATE INDEX IF NOT EXISTS idx_pipelines_next_run_at ON pipelines(next_run_at);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_retry_of_run ON pipeline_runs(retry_of_run_id);

CREATE TABLE IF NOT EXISTS column_lineage_edges (
    id                UUID PRIMARY KEY,
    source_dataset_id UUID NOT NULL,
    source_column     TEXT NOT NULL,
    target_dataset_id UUID NOT NULL,
    target_column     TEXT NOT NULL,
    pipeline_id       UUID REFERENCES pipelines(id) ON DELETE SET NULL,
    node_id           TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_dataset_id, source_column, target_dataset_id, target_column, pipeline_id, node_id)
);

CREATE INDEX IF NOT EXISTS idx_column_lineage_source ON column_lineage_edges(source_dataset_id, source_column);
CREATE INDEX IF NOT EXISTS idx_column_lineage_target ON column_lineage_edges(target_dataset_id, target_column);