CREATE TABLE IF NOT EXISTS ontology_interfaces (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS interface_properties (
    id UUID PRIMARY KEY,
    interface_id UUID NOT NULL REFERENCES ontology_interfaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    property_type TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT FALSE,
    unique_constraint BOOLEAN NOT NULL DEFAULT FALSE,
    time_dependent BOOLEAN NOT NULL DEFAULT FALSE,
    default_value JSONB,
    validation_rules JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (interface_id, name)
);

CREATE TABLE IF NOT EXISTS object_type_interfaces (
    object_type_id UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    interface_id UUID NOT NULL REFERENCES ontology_interfaces(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (object_type_id, interface_id)
);

ALTER TABLE properties
    ADD COLUMN IF NOT EXISTS time_dependent BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE object_instances
    ADD COLUMN IF NOT EXISTS organization_id UUID;

ALTER TABLE object_instances
    ADD COLUMN IF NOT EXISTS marking TEXT NOT NULL DEFAULT 'public';

CREATE INDEX IF NOT EXISTS idx_interface_properties_interface
    ON interface_properties(interface_id);

CREATE INDEX IF NOT EXISTS idx_object_type_interfaces_object_type
    ON object_type_interfaces(object_type_id);

CREATE INDEX IF NOT EXISTS idx_object_instances_organization
    ON object_instances(organization_id);

CREATE INDEX IF NOT EXISTS idx_object_instances_marking
    ON object_instances(marking);
