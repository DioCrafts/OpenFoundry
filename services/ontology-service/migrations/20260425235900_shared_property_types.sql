CREATE TABLE IF NOT EXISTS shared_property_types (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    property_type TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT FALSE,
    unique_constraint BOOLEAN NOT NULL DEFAULT FALSE,
    time_dependent BOOLEAN NOT NULL DEFAULT FALSE,
    default_value JSONB,
    validation_rules JSONB,
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS object_type_shared_property_types (
    object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    shared_property_type_id UUID NOT NULL REFERENCES shared_property_types(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (object_type_id, shared_property_type_id)
);

CREATE INDEX IF NOT EXISTS idx_shared_property_types_property_type
    ON shared_property_types(property_type);

CREATE INDEX IF NOT EXISTS idx_object_type_shared_property_types_object_type
    ON object_type_shared_property_types(object_type_id);

CREATE INDEX IF NOT EXISTS idx_object_type_shared_property_types_shared_property
    ON object_type_shared_property_types(shared_property_type_id);
