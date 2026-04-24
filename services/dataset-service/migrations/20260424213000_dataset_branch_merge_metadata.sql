ALTER TABLE dataset_branches
    ADD COLUMN IF NOT EXISTS base_version INT NOT NULL DEFAULT 1;

UPDATE dataset_branches
SET base_version = version
WHERE base_version IS NULL OR base_version <= 0;
