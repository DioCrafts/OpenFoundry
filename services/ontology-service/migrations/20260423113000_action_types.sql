CREATE TABLE IF NOT EXISTS action_types (
    id                    UUID PRIMARY KEY,
    name                  TEXT NOT NULL UNIQUE,
    display_name          TEXT NOT NULL,
    description           TEXT NOT NULL DEFAULT '',
    object_type_id        UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    operation_kind        TEXT NOT NULL,
    input_schema          JSONB NOT NULL DEFAULT '[]'::jsonb,
    config                JSONB NOT NULL DEFAULT 'null'::jsonb,
    confirmation_required BOOLEAN NOT NULL DEFAULT FALSE,
    permission_key        TEXT,
    owner_id              UUID NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_action_types_object_type ON action_types(object_type_id);
CREATE INDEX IF NOT EXISTS idx_action_types_operation_kind ON action_types(operation_kind);