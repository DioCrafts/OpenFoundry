ALTER TABLE marketplace_product_fleets
    ADD COLUMN IF NOT EXISTS deployment_cells JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS residency_policy JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS marketplace_fleet_promotion_gates (
    id UUID PRIMARY KEY,
    fleet_id UUID NOT NULL REFERENCES marketplace_product_fleets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gate_kind TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT TRUE,
    status TEXT NOT NULL DEFAULT 'pending',
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    notes TEXT NOT NULL DEFAULT '',
    last_evaluated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT marketplace_fleet_promotion_gates_status_valid
        CHECK (status IN ('pending', 'passed', 'failed', 'waived')),
    UNIQUE (fleet_id, name)
);

CREATE INDEX IF NOT EXISTS idx_marketplace_fleet_promotion_gates_fleet
    ON marketplace_fleet_promotion_gates(fleet_id, updated_at DESC);

INSERT INTO marketplace_fleet_promotion_gates (
    id,
    fleet_id,
    name,
    gate_kind,
    required,
    status,
    evidence,
    notes,
    last_evaluated_at,
    created_at,
    updated_at
)
VALUES
    (
        '01968d50-2f20-71b8-a6f0-0f4000002201',
        '01968c10-2f20-71b8-a6f0-0f4000002001',
        'artifact-integrity',
        'supply-chain',
        TRUE,
        'passed',
        jsonb_build_object('provenance', 'signed-manifest', 'verified_at', NOW()),
        'Package manifest and packaged resources were verified before rollout.',
        NOW() - interval '30 minutes',
        NOW() - interval '2 days',
        NOW() - interval '30 minutes'
    ),
    (
        '01968d50-2f20-71b8-a6f0-0f4000002202',
        '01968c10-2f20-71b8-a6f0-0f4000002001',
        'observability-burn-in',
        'synthetic-check',
        TRUE,
        'pending',
        jsonb_build_object('dashboard', 'ops-center-rollout'),
        'Awaiting fresh burn-in metrics from the canary environment.',
        NOW() - interval '15 minutes',
        NOW() - interval '2 days',
        NOW() - interval '15 minutes'
    )
ON CONFLICT (fleet_id, name) DO NOTHING;
