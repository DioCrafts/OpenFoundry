CREATE TABLE IF NOT EXISTS report_definitions (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    owner TEXT NOT NULL,
    generator_kind TEXT NOT NULL,
    dataset_name TEXT NOT NULL,
    template JSONB NOT NULL DEFAULT '{}'::jsonb,
    schedule JSONB NOT NULL DEFAULT '{}'::jsonb,
    recipients JSONB NOT NULL DEFAULT '[]'::jsonb,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    last_generated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS report_executions (
    id UUID PRIMARY KEY,
    report_id UUID NOT NULL REFERENCES report_definitions(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    generator_kind TEXT NOT NULL,
    triggered_by TEXT NOT NULL DEFAULT 'system',
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    preview JSONB NOT NULL DEFAULT '{}'::jsonb,
    artifact JSONB NOT NULL DEFAULT '{}'::jsonb,
    distributions JSONB NOT NULL DEFAULT '[]'::jsonb,
    metrics JSONB NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO report_definitions (
    id,
    name,
    description,
    owner,
    generator_kind,
    dataset_name,
    template,
    schedule,
    recipients,
    tags,
    parameters,
    active,
    last_generated_at,
    created_at,
    updated_at
) VALUES (
    '0196839d-b500-7b3a-8a1d-7ab001010001',
    'Executive Revenue Pulse',
    'Weekly executive digest with commercial KPIs, regional split, and geospatial hotspots.',
    'Revenue Operations',
    'pdf',
    'sales_fact_daily',
    jsonb_build_object(
        'title', 'Executive Revenue Pulse',
        'subtitle', 'Weekly commercial operating review',
        'theme', 'copper',
        'layout', 'briefing',
        'sections', jsonb_build_array(
            jsonb_build_object('id', 'kpi-margin', 'title', 'Gross Margin', 'kind', 'kpi', 'query', 'select margin from revenue_kpis', 'description', 'Margin headline', 'config', jsonb_build_object('unit', '%')),
            jsonb_build_object('id', 'regional-table', 'title', 'Regional Revenue', 'kind', 'table', 'query', 'select region, revenue from regional_revenue', 'description', 'Regional split', 'config', jsonb_build_object('sortBy', 'value')),
            jsonb_build_object('id', 'map-hotspots', 'title', 'Pipeline Hotspots', 'kind', 'map', 'query', 'select lat, lon, value from opportunities', 'description', 'Geo coverage', 'config', jsonb_build_object('layer', 'heatmap'))
        )
    ),
    jsonb_build_object(
        'cadence', 'weekly',
        'expression', '0 9 * * MON',
        'timezone', 'UTC',
        'anchor_time', '09:00',
        'interval_minutes', 10080,
        'enabled', true,
        'next_run_at', to_jsonb(NOW() + interval '6 days')
    ),
    jsonb_build_array(
        jsonb_build_object('id', 'exec-email', 'channel', 'email', 'target', 'exec-team@openfoundry.dev', 'label', 'Executive distribution', 'config', jsonb_build_object('subject', 'Weekly revenue pulse')),
        jsonb_build_object('id', 'revops-slack', 'channel', 'slack', 'target', '#revops', 'label', 'RevOps room', 'config', jsonb_build_object('webhook', 'revops-webhook')),
        jsonb_build_object('id', 'archive', 'channel', 's3', 'target', 's3://openfoundry-reports/executive', 'label', 'Archive', 'config', jsonb_build_object('prefix', 'weekly'))
    ),
    jsonb_build_array('executive', 'weekly', 'revenue'),
    jsonb_build_object('filters', jsonb_build_object('region', 'all', 'currency', 'USD')),
    true,
    NOW() - interval '2 days',
    NOW() - interval '30 days',
    NOW() - interval '2 days'
), (
    '0196839d-b500-7b3a-8a1d-7ab001010002',
    'Operations SLA Scorecard',
    'Daily operational report combining SLA drift, ticket queues, and service exceptions.',
    'Platform Operations',
    'excel',
    'incident_response_fact',
    jsonb_build_object(
        'title', 'Operations SLA Scorecard',
        'subtitle', 'Daily operating review',
        'theme', 'slate',
        'layout', 'scorecard',
        'sections', jsonb_build_array(
            jsonb_build_object('id', 'sla-kpi', 'title', 'SLA Compliance', 'kind', 'kpi', 'query', 'select compliance from sla_kpis', 'description', 'Topline compliance', 'config', jsonb_build_object('unit', '%')),
            jsonb_build_object('id', 'ticket-trend', 'title', 'Ticket Trend', 'kind', 'chart', 'query', 'select day, opened from ticket_trend', 'description', 'Trendline', 'config', jsonb_build_object('chart', 'line')),
            jsonb_build_object('id', 'ops-commentary', 'title', 'Analyst Notes', 'kind', 'narrative', 'query', 'select note from analyst_notes', 'description', 'Narrative context', 'config', jsonb_build_object())
        )
    ),
    jsonb_build_object(
        'cadence', 'daily',
        'expression', '0 7 * * *',
        'timezone', 'UTC',
        'anchor_time', '07:00',
        'interval_minutes', 1440,
        'enabled', true,
        'next_run_at', to_jsonb(NOW() + interval '18 hours')
    ),
    jsonb_build_array(
        jsonb_build_object('id', 'ops-email', 'channel', 'email', 'target', 'ops@openfoundry.dev', 'label', 'Ops digest', 'config', jsonb_build_object('subject', 'Daily SLA scorecard')),
        jsonb_build_object('id', 'incident-webhook', 'channel', 'webhook', 'target', 'https://hooks.openfoundry.dev/reporting/ops', 'label', 'Ops automation', 'config', jsonb_build_object('secret', 'local-dev'))
    ),
    jsonb_build_array('ops', 'daily', 'sla'),
    jsonb_build_object('filters', jsonb_build_object('severity', 'all', 'environment', 'prod')),
    true,
    NOW() - interval '12 hours',
    NOW() - interval '20 days',
    NOW() - interval '12 hours'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO report_executions (
    id,
    report_id,
    status,
    generator_kind,
    triggered_by,
    generated_at,
    completed_at,
    preview,
    artifact,
    distributions,
    metrics
) VALUES (
    '0196839d-b500-7b3a-8a1d-7ab001010101',
    '0196839d-b500-7b3a-8a1d-7ab001010001',
    'completed',
    'pdf',
    'scheduler',
    NOW() - interval '2 days',
    NOW() - interval '2 days' + interval '3 minutes',
    jsonb_build_object(
        'headline', 'Executive Revenue Pulse generated for Revenue Operations',
        'generated_for', 'sales_fact_daily',
        'engine', 'typst',
        'highlights', jsonb_build_array(
            jsonb_build_object('label', 'Rows scanned', 'value', '18k', 'delta', '+4%'),
            jsonb_build_object('label', 'Freshness', 'value', '8 min', 'delta', 'within SLA'),
            jsonb_build_object('label', 'Exception rate', 'value', '2%', 'delta', 'stable')
        ),
        'sections', jsonb_build_array(
            jsonb_build_object('section_id', 'kpi-margin', 'title', 'Gross Margin', 'kind', 'kpi', 'summary', 'Gross Margin is holding above threshold.', 'rows', jsonb_build_array(jsonb_build_object('metric', 'Gross Margin', 'value', 87, 'target', 90, 'unit', '%'))),
            jsonb_build_object('section_id', 'regional-table', 'title', 'Regional Revenue', 'kind', 'table', 'summary', 'Regional split is concentrated in North America and Europe.', 'rows', jsonb_build_array(jsonb_build_object('region', 'North America', 'value', 182000, 'variance', '+4.2%'), jsonb_build_object('region', 'Europe', 'value', 149000, 'variance', '+2.1%')))
        )
    ),
    jsonb_build_object(
        'file_name', 'executive-revenue-pulse-202604201000.pdf',
        'mime_type', 'application/pdf',
        'size_bytes', 182400,
        'storage_url', '/api/v1/reports/executions/0196839d-b500-7b3a-8a1d-7ab001010101/download',
        'checksum', '0196839d-b500-7b3a-8a1d-7ab001010101-pdf'
    ),
    jsonb_build_array(
        jsonb_build_object('channel', 'email', 'target', 'exec-team@openfoundry.dev', 'status', 'delivered', 'delivered_at', to_jsonb(NOW() - interval '2 days' + interval '4 minutes'), 'detail', 'Queued Executive Revenue Pulse for exec-team@openfoundry.dev'),
        jsonb_build_object('channel', 'slack', 'target', '#revops', 'status', 'delivered', 'delivered_at', to_jsonb(NOW() - interval '2 days' + interval '5 minutes'), 'detail', 'Posted digest card to #revops')
    ),
    jsonb_build_object('duration_ms', 1320, 'row_count', 9, 'section_count', 3, 'recipient_count', 3)
)
ON CONFLICT (id) DO NOTHING;