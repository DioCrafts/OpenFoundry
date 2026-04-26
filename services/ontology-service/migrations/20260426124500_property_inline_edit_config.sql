ALTER TABLE properties
ADD COLUMN IF NOT EXISTS inline_edit_config JSONB;
