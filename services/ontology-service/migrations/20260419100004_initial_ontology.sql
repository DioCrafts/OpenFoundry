-- Ontology: object types, properties, link types, instances

CREATE TABLE IF NOT EXISTS object_types (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    primary_key_property TEXT,
    icon        TEXT,
    color       TEXT,
    owner_id    UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS properties (
    id               UUID PRIMARY KEY,
    object_type_id   UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    display_name     TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    property_type    TEXT NOT NULL,
    required         BOOLEAN NOT NULL DEFAULT FALSE,
    unique_constraint BOOLEAN NOT NULL DEFAULT FALSE,
    default_value    JSONB,
    validation_rules JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (object_type_id, name)
);

CREATE TABLE IF NOT EXISTS link_types (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    source_type_id  UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    target_type_id  UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    cardinality     TEXT NOT NULL DEFAULT 'many_to_many',
    owner_id        UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (name, source_type_id, target_type_id)
);

CREATE TABLE IF NOT EXISTS object_instances (
    id              UUID PRIMARY KEY,
    object_type_id  UUID NOT NULL REFERENCES object_types(id) ON DELETE CASCADE,
    properties      JSONB NOT NULL DEFAULT '{}',
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS link_instances (
    id                UUID PRIMARY KEY,
    link_type_id      UUID NOT NULL REFERENCES link_types(id) ON DELETE CASCADE,
    source_object_id  UUID NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    target_object_id  UUID NOT NULL REFERENCES object_instances(id) ON DELETE CASCADE,
    properties        JSONB,
    created_by        UUID NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_properties_object_type ON properties(object_type_id);
CREATE INDEX idx_link_types_source ON link_types(source_type_id);
CREATE INDEX idx_link_types_target ON link_types(target_type_id);
CREATE INDEX idx_object_instances_type ON object_instances(object_type_id);
CREATE INDEX idx_link_instances_type ON link_instances(link_type_id);
CREATE INDEX idx_link_instances_source ON link_instances(source_object_id);
CREATE INDEX idx_link_instances_target ON link_instances(target_object_id);
