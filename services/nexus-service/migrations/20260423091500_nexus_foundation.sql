CREATE TABLE IF NOT EXISTS nexus_peers (
	id UUID PRIMARY KEY,
	slug TEXT NOT NULL UNIQUE,
	display_name TEXT NOT NULL,
	region TEXT NOT NULL,
	endpoint_url TEXT NOT NULL,
	auth_mode TEXT NOT NULL,
	trust_level TEXT NOT NULL,
	public_key_fingerprint TEXT NOT NULL,
	shared_scopes JSONB NOT NULL DEFAULT '[]'::jsonb,
	status TEXT NOT NULL,
	last_handshake_at TIMESTAMPTZ,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nexus_contracts (
	id UUID PRIMARY KEY,
	peer_id UUID NOT NULL REFERENCES nexus_peers(id) ON DELETE CASCADE,
	name TEXT NOT NULL,
	description TEXT NOT NULL,
	dataset_locator TEXT NOT NULL,
	allowed_purposes JSONB NOT NULL DEFAULT '[]'::jsonb,
	data_classes JSONB NOT NULL DEFAULT '[]'::jsonb,
	residency_region TEXT NOT NULL,
	query_template TEXT NOT NULL,
	max_rows_per_query BIGINT NOT NULL,
	replication_mode TEXT NOT NULL,
	encryption_profile TEXT NOT NULL,
	retention_days INTEGER NOT NULL,
	status TEXT NOT NULL,
	signed_at TIMESTAMPTZ,
	expires_at TIMESTAMPTZ NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nexus_shares (
	id UUID PRIMARY KEY,
	contract_id UUID NOT NULL REFERENCES nexus_contracts(id) ON DELETE CASCADE,
	provider_peer_id UUID NOT NULL REFERENCES nexus_peers(id) ON DELETE CASCADE,
	consumer_peer_id UUID NOT NULL REFERENCES nexus_peers(id) ON DELETE CASCADE,
	dataset_name TEXT NOT NULL,
	selector JSONB NOT NULL DEFAULT '{}'::jsonb,
	provider_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
	consumer_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
	sample_rows JSONB NOT NULL DEFAULT '[]'::jsonb,
	replication_mode TEXT NOT NULL,
	status TEXT NOT NULL,
	last_sync_at TIMESTAMPTZ,
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nexus_access_grants (
	id UUID PRIMARY KEY,
	share_id UUID NOT NULL REFERENCES nexus_shares(id) ON DELETE CASCADE,
	peer_id UUID NOT NULL REFERENCES nexus_peers(id) ON DELETE CASCADE,
	query_template TEXT NOT NULL,
	max_rows_per_query BIGINT NOT NULL,
	can_replicate BOOLEAN NOT NULL DEFAULT FALSE,
	allowed_purposes JSONB NOT NULL DEFAULT '[]'::jsonb,
	expires_at TIMESTAMPTZ NOT NULL,
	issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nexus_sync_statuses (
	id UUID PRIMARY KEY,
	share_id UUID NOT NULL UNIQUE REFERENCES nexus_shares(id) ON DELETE CASCADE,
	mode TEXT NOT NULL,
	status TEXT NOT NULL,
	rows_replicated BIGINT NOT NULL DEFAULT 0,
	backlog_rows BIGINT NOT NULL DEFAULT 0,
	encrypted_in_transit BOOLEAN NOT NULL DEFAULT TRUE,
	encrypted_at_rest BOOLEAN NOT NULL DEFAULT TRUE,
	key_version TEXT NOT NULL,
	last_sync_at TIMESTAMPTZ,
	next_sync_at TIMESTAMPTZ,
	audit_cursor TEXT NOT NULL,
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO nexus_peers (id, slug, display_name, region, endpoint_url, auth_mode, trust_level, public_key_fingerprint, shared_scopes, status, last_handshake_at, created_at, updated_at)
VALUES
	('019687c4-17fc-7e7d-9dd4-b3cf85a1b001', 'acme-health', 'Acme Health', 'eu-west-1', 'https://nexus.acme-health.example', 'mtls+jwt', 'trusted', 'SHA256:8A:2F:11:ACME', '["claims", "coverage", "audit"]'::jsonb, 'authenticated', NOW() - INTERVAL '2 hours', NOW() - INTERVAL '7 days', NOW() - INTERVAL '2 hours'),
	('019687c4-17fc-7e7d-9dd4-b3cf85a1b002', 'northwind-retail', 'Northwind Retail', 'us-east-1', 'https://exchange.northwind.example', 'oidc+mtls', 'partner', 'SHA256:9B:3C:22:NW', '["inventory", "orders"]'::jsonb, 'authenticated', NOW() - INTERVAL '5 hours', NOW() - INTERVAL '10 days', NOW() - INTERVAL '5 hours')
ON CONFLICT (id) DO NOTHING;

INSERT INTO nexus_contracts (id, peer_id, name, description, dataset_locator, allowed_purposes, data_classes, residency_region, query_template, max_rows_per_query, replication_mode, encryption_profile, retention_days, status, signed_at, expires_at, created_at, updated_at)
VALUES
	('019687c4-17fc-7e7d-9dd4-b3cf85a1c001', '019687c4-17fc-7e7d-9dd4-b3cf85a1b001', 'Claims Federated Access', 'Cross-org access to adjudicated claims with residency and purpose controls.', 'acme-health://claims/adjudicated', '["claims-investigation", "member-support"]'::jsonb, '["pii", "confidential"]'::jsonb, 'eu', 'SELECT * FROM claims WHERE member_region = ''EU''', 2500, 'incremental_replication', 'mutual-tls+envelope', 365, 'active', NOW() - INTERVAL '14 days', NOW() + INTERVAL '180 days', NOW() - INTERVAL '20 days', NOW() - INTERVAL '1 day'),
	('019687c4-17fc-7e7d-9dd4-b3cf85a1c002', '019687c4-17fc-7e7d-9dd4-b3cf85a1b002', 'Inventory Snapshot Exchange', 'Selective inventory snapshots shared for supplier fulfillment workflows.', 'northwind-retail://inventory/warehouse', '["supplier-fulfillment", "demand-planning"]'::jsonb, '["confidential"]'::jsonb, 'us', 'SELECT sku, quantity, warehouse_id FROM inventory_snapshot', 5000, 'query_only', 'mutual-tls+envelope', 180, 'active', NOW() - INTERVAL '30 days', NOW() + INTERVAL '365 days', NOW() - INTERVAL '32 days', NOW() - INTERVAL '1 hour')
ON CONFLICT (id) DO NOTHING;

INSERT INTO nexus_shares (id, contract_id, provider_peer_id, consumer_peer_id, dataset_name, selector, provider_schema, consumer_schema, sample_rows, replication_mode, status, last_sync_at, created_at, updated_at)
VALUES
	('019687c4-17fc-7e7d-9dd4-b3cf85a1d001', '019687c4-17fc-7e7d-9dd4-b3cf85a1c001', '019687c4-17fc-7e7d-9dd4-b3cf85a1b001', '019687c4-17fc-7e7d-9dd4-b3cf85a1b002', 'adjudicated_claims_eu', '{"member_region": "EU", "updated_since": "P7D"}'::jsonb, '{"claim_id": "string", "member_id": "string", "amount": "number", "status": "string"}'::jsonb, '{"claim_id": "string", "member_id": "string", "amount": "number", "status": "string"}'::jsonb, '[{"claim_id": "CLM-1001", "member_id": "M-22", "amount": 1200.55, "status": "approved"}, {"claim_id": "CLM-1002", "member_id": "M-18", "amount": 318.40, "status": "pending"}]'::jsonb, 'incremental_replication', 'active', NOW() - INTERVAL '45 minutes', NOW() - INTERVAL '7 days', NOW() - INTERVAL '45 minutes'),
	('019687c4-17fc-7e7d-9dd4-b3cf85a1d002', '019687c4-17fc-7e7d-9dd4-b3cf85a1c002', '019687c4-17fc-7e7d-9dd4-b3cf85a1b002', '019687c4-17fc-7e7d-9dd4-b3cf85a1b001', 'warehouse_snapshot', '{"warehouse_id": ["AMS-1", "MAD-2"]}'::jsonb, '{"sku": "string", "quantity": "number", "warehouse_id": "string"}'::jsonb, '{"sku": "string", "quantity": "number", "warehouse_id": "string", "last_restock_at": "string"}'::jsonb, '[{"sku": "AX-100", "quantity": 28, "warehouse_id": "AMS-1"}, {"sku": "AX-101", "quantity": 54, "warehouse_id": "MAD-2"}]'::jsonb, 'query_only', 'active', NOW() - INTERVAL '2 hours', NOW() - INTERVAL '14 days', NOW() - INTERVAL '2 hours')
ON CONFLICT (id) DO NOTHING;

INSERT INTO nexus_access_grants (id, share_id, peer_id, query_template, max_rows_per_query, can_replicate, allowed_purposes, expires_at, issued_at)
VALUES
	('019687c4-17fc-7e7d-9dd4-b3cf85a1e001', '019687c4-17fc-7e7d-9dd4-b3cf85a1d001', '019687c4-17fc-7e7d-9dd4-b3cf85a1b002', 'SELECT claim_id, member_id, amount, status FROM adjudicated_claims_eu', 2500, TRUE, '["claims-investigation", "member-support"]'::jsonb, NOW() + INTERVAL '180 days', NOW() - INTERVAL '14 days'),
	('019687c4-17fc-7e7d-9dd4-b3cf85a1e002', '019687c4-17fc-7e7d-9dd4-b3cf85a1d002', '019687c4-17fc-7e7d-9dd4-b3cf85a1b001', 'SELECT sku, quantity, warehouse_id FROM warehouse_snapshot', 5000, FALSE, '["supplier-fulfillment", "demand-planning"]'::jsonb, NOW() + INTERVAL '365 days', NOW() - INTERVAL '30 days')
ON CONFLICT (id) DO NOTHING;

INSERT INTO nexus_sync_statuses (id, share_id, mode, status, rows_replicated, backlog_rows, encrypted_in_transit, encrypted_at_rest, key_version, last_sync_at, next_sync_at, audit_cursor, updated_at)
VALUES
	('019687c4-17fc-7e7d-9dd4-b3cf85a1f001', '019687c4-17fc-7e7d-9dd4-b3cf85a1d001', 'incremental_replication', 'ready', 18420, 320, TRUE, TRUE, 'key/eu-prod/v4', NOW() - INTERVAL '45 minutes', NOW() + INTERVAL '15 minutes', 'cursor/acme-claims/2026-04-23T08:15:00Z', NOW() - INTERVAL '10 minutes'),
	('019687c4-17fc-7e7d-9dd4-b3cf85a1f002', '019687c4-17fc-7e7d-9dd4-b3cf85a1d002', 'query_only', 'degraded', 0, 0, TRUE, TRUE, 'key/us-prod/v2', NOW() - INTERVAL '2 hours', NOW() + INTERVAL '6 hours', 'cursor/northwind-inventory/2026-04-23T06:05:00Z', NOW() - INTERVAL '5 minutes')
ON CONFLICT (id) DO NOTHING;