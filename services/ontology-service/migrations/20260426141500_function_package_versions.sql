ALTER TABLE ontology_function_packages
    ADD COLUMN IF NOT EXISTS version TEXT NOT NULL DEFAULT '0.1.0';

UPDATE ontology_function_packages
SET version = '0.1.0'
WHERE version IS NULL OR BTRIM(version) = '';

ALTER TABLE ontology_function_packages
    DROP CONSTRAINT IF EXISTS ontology_function_packages_name_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ontology_function_packages_name_version
    ON ontology_function_packages(name, version);

CREATE INDEX IF NOT EXISTS idx_ontology_function_packages_name
    ON ontology_function_packages(name);
