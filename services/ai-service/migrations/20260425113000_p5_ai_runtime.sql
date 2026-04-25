ALTER TABLE ai_tools
ADD COLUMN IF NOT EXISTS execution_config JSONB NOT NULL DEFAULT '{}'::jsonb;
