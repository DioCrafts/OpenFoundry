ALTER TABLE sync_jobs
    ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS max_attempts INT NOT NULL DEFAULT 3,
    ADD COLUMN IF NOT EXISTS scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS result_dataset_version INT,
    ADD COLUMN IF NOT EXISTS sync_metadata JSONB NOT NULL DEFAULT '{}';

UPDATE sync_jobs
SET scheduled_at = created_at
WHERE scheduled_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_sync_jobs_status_schedule
    ON sync_jobs(status, COALESCE(next_retry_at, scheduled_at, created_at));
