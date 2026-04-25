ALTER TABLE ml_experiments
    ADD COLUMN IF NOT EXISTS objective_spec JSONB NOT NULL DEFAULT '{}'::jsonb;
