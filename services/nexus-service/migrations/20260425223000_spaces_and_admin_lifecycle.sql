ALTER TABLE nexus_peers
    ADD COLUMN IF NOT EXISTS organization_type TEXT NOT NULL DEFAULT 'partner',
    ADD COLUMN IF NOT EXISTS lifecycle_stage TEXT NOT NULL DEFAULT 'onboarding',
    ADD COLUMN IF NOT EXISTS admin_contacts JSONB NOT NULL DEFAULT '[]'::jsonb;

UPDATE nexus_peers
SET organization_type = CASE
        WHEN slug LIKE 'acme-%' THEN 'host'
        ELSE 'partner'
    END,
    lifecycle_stage = CASE
        WHEN status = 'authenticated' THEN 'active'
        ELSE 'onboarding'
    END,
    admin_contacts = CASE
        WHEN jsonb_typeof(admin_contacts) = 'array'
            AND jsonb_array_length(admin_contacts) > 0 THEN admin_contacts
        ELSE jsonb_build_array(
            lower(replace(slug, '-', '.')) || '@example.com'
        )
    END;

CREATE TABLE IF NOT EXISTS nexus_spaces (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    space_kind TEXT NOT NULL,
    owner_peer_id UUID REFERENCES nexus_peers(id) ON DELETE SET NULL,
    region TEXT NOT NULL,
    member_peer_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    governance_tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE nexus_shares
    ADD COLUMN IF NOT EXISTS provider_space_id UUID REFERENCES nexus_spaces(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS consumer_space_id UUID REFERENCES nexus_spaces(id) ON DELETE SET NULL;

INSERT INTO nexus_spaces (id, slug, display_name, description, space_kind, owner_peer_id, region, member_peer_ids, governance_tags, status, created_at, updated_at)
VALUES
    (
        '019dc5aa-e101-7b9a-8c12-e131a1110001',
        'host-private-ops',
        'Host Private Ops',
        'Private host-only coordination space for regulated rollout and exchange ops.',
        'private',
        '019687c4-17fc-7e7d-9dd4-b3cf85a1b001',
        'eu-west-1',
        '["019687c4-17fc-7e7d-9dd4-b3cf85a1b001"]'::jsonb,
        '["regulated", "ops"]'::jsonb,
        'active',
        NOW() - INTERVAL '7 days',
        NOW() - INTERVAL '2 hours'
    ),
    (
        '019dc5aa-e101-7b9a-8c12-e131a1110002',
        'partner-shared-exchange',
        'Partner Shared Exchange',
        'Shared cross-org space for contracts, curated datasets and partner onboarding.',
        'shared',
        '019687c4-17fc-7e7d-9dd4-b3cf85a1b001',
        'eu-west-1',
        '["019687c4-17fc-7e7d-9dd4-b3cf85a1b001", "019687c4-17fc-7e7d-9dd4-b3cf85a1b002"]'::jsonb,
        '["partners", "shared-data"]'::jsonb,
        'active',
        NOW() - INTERVAL '6 days',
        NOW() - INTERVAL '30 minutes'
    )
ON CONFLICT (id) DO NOTHING;

UPDATE nexus_shares
SET provider_space_id = '019dc5aa-e101-7b9a-8c12-e131a1110001',
    consumer_space_id = '019dc5aa-e101-7b9a-8c12-e131a1110002'
WHERE id = '019687c4-17fc-7e7d-9dd4-b3cf85a1d001'
  AND provider_space_id IS NULL
  AND consumer_space_id IS NULL;

UPDATE nexus_shares
SET provider_space_id = '019dc5aa-e101-7b9a-8c12-e131a1110002',
    consumer_space_id = '019dc5aa-e101-7b9a-8c12-e131a1110001'
WHERE id = '019687c4-17fc-7e7d-9dd4-b3cf85a1d002'
  AND provider_space_id IS NULL
  AND consumer_space_id IS NULL;
