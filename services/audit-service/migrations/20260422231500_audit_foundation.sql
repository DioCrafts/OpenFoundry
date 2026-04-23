CREATE TABLE IF NOT EXISTS audit_events (
    id UUID PRIMARY KEY,
    sequence BIGINT NOT NULL UNIQUE,
    previous_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL,
    source_service TEXT NOT NULL,
    channel TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    status TEXT NOT NULL,
    severity TEXT NOT NULL,
    classification TEXT NOT NULL,
    subject_id TEXT,
    ip_address TEXT,
    location TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    labels JSONB NOT NULL DEFAULT '[]'::jsonb,
    retention_until TIMESTAMPTZ NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT audit_events_append_only CHECK (sequence > 0)
);

CREATE TABLE IF NOT EXISTS audit_policies (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    scope TEXT NOT NULL,
    classification TEXT NOT NULL,
    retention_days INTEGER NOT NULL,
    legal_hold BOOLEAN NOT NULL DEFAULT FALSE,
    purge_mode TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    rules JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS compliance_reports (
    id UUID PRIMARY KEY,
    standard TEXT NOT NULL,
    title TEXT NOT NULL,
    scope TEXT NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    findings JSONB NOT NULL DEFAULT '[]'::jsonb,
    artifact JSONB NOT NULL DEFAULT '{}'::jsonb,
    relevant_event_count BIGINT NOT NULL DEFAULT 0,
    policy_count BIGINT NOT NULL DEFAULT 0,
    control_summary TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

INSERT INTO audit_events (id, sequence, previous_hash, entry_hash, source_service, channel, actor, action, resource_type, resource_id, status, severity, classification, subject_id, ip_address, location, metadata, labels, retention_until, occurred_at, ingested_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001050001',
    1,
    'GENESIS',
    'AUD-00000001-GENESIS-GATEWAYR',
    'gateway',
    'http',
    'system:gateway',
    'request.forwarded',
    'http_request',
    '/api/v1/reports/definitions',
    'success',
    'low',
    'confidential',
    'subject-demo-1',
    '10.0.0.21',
    'Madrid',
    jsonb_build_object('method', 'GET', 'route', '/api/v1/reports/definitions', 'nats_subject', 'of.audit.gateway'),
    jsonb_build_array('auto-captured', 'phase4', 'gateway'),
    NOW() + interval '365 days',
    NOW() - interval '4 hours',
    NOW() - interval '4 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001050002',
    2,
    'AUD-00000001-GENESIS-GATEWAYR',
    'AUD-00000002-AUD00000-AUTHSERV',
    'auth-service',
    'nats',
    'user:marco',
    'session.token.issued',
    'session',
    'sess_98412',
    'success',
    'medium',
    'pii',
    'subject-demo-1',
    '10.0.0.45',
    'Barcelona',
    jsonb_build_object('nats_subject', 'of.audit.auth', 'device', 'managed-laptop'),
    jsonb_build_array('auto-captured', 'contains-sensitive-data'),
    NOW() + interval '730 days',
    NOW() - interval '3 hours',
    NOW() - interval '3 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001050003',
    3,
    'AUD-00000002-AUD00000-AUTHSERV',
    'AUD-00000003-AUD00000-DATASETS',
    'dataset-service',
    'nats',
    'service:ingestion',
    'dataset.exported',
    'dataset',
    'sales_fact_daily',
    'success',
    'critical',
    'pii',
    'subject-demo-2',
    '10.0.0.52',
    'Valencia',
    jsonb_build_object('rows', 120045, 'destination', 'analytics-s3', 'nats_subject', 'of.audit.datasets'),
    jsonb_build_array('auto-captured', 'export', 'contains-sensitive-data'),
    NOW() + interval '1095 days',
    NOW() - interval '95 minutes',
    NOW() - interval '95 minutes'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO audit_policies (id, name, description, scope, classification, retention_days, legal_hold, purge_mode, active, rules, updated_by, created_at, updated_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001050101',
    'PII Access Retention',
    'Retain PII-linked access and export events for two years with legal hold hooks.',
    'identity-and-access',
    'pii',
    730,
    true,
    'redact-then-retain-hash',
    true,
    jsonb_build_array('mask subject payloads on erasure', 'preserve entry hash chain', 'weekly legal hold review'),
    'Security Governance',
    NOW() - interval '12 days',
    NOW() - interval '6 hours'
),
(
    '0196839d-d210-7f8c-8a1d-7ab001050102',
    'Operational Logs TTL',
    'Purge low-sensitivity operational events after one year.',
    'operations',
    'public',
    365,
    false,
    'hard-delete-after-ttl',
    true,
    jsonb_build_array('delete after TTL', 'notify platform ops on backlog > 1000'),
    'Platform Ops',
    NOW() - interval '8 days',
    NOW() - interval '3 hours'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO compliance_reports (id, standard, title, scope, window_start, window_end, generated_at, status, findings, artifact, relevant_event_count, policy_count, control_summary, expires_at)
VALUES
(
    '0196839d-d210-7f8c-8a1d-7ab001050201',
    'soc2',
    'SOC2 Q2 Evidence Pack',
    'production-platform',
    NOW() - interval '90 days',
    NOW(),
    NOW() - interval '2 hours',
    'ready',
    jsonb_build_array(
        jsonb_build_object('control_id', 'CC7.2', 'title', 'Access monitoring in place', 'status', 'pass', 'evidence', 'Immutable request audit chain across gateway and auth events.'),
        jsonb_build_object('control_id', 'CC8.1', 'title', 'Retention policy defined', 'status', 'pass', 'evidence', 'Active retention TTL policies cover PII and operational logs.')
    ),
    jsonb_build_object('file_name', 'soc2-q2.zip', 'mime_type', 'application/zip', 'storage_url', 's3://compliance-reports/production-platform/soc2-q2.zip', 'checksum', 'cmp-soc2-q2', 'size_bytes', 48930),
    3,
    2,
    '2 controls evidenced across 3 events',
    NOW() + interval '30 days'
)
ON CONFLICT (id) DO NOTHING;