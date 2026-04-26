ALTER TABLE action_types
ADD COLUMN IF NOT EXISTS form_schema JSONB NOT NULL DEFAULT '{}'::jsonb;
