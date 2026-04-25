INSERT INTO ai_tools (
    id,
    name,
    description,
    category,
    execution_mode,
    execution_config,
    status,
    input_schema,
    output_schema,
    tags
) VALUES
(
    '01969888-3550-70c0-9000-000000000033',
    'Dataset Navigator',
    'Plans governed dataset preview, lint, branch and export operations inside the native agent runtime.',
    'data',
    'native_dataset',
    '{"default_dataset_name":"customer_health","branch_prefix":"what-if","sensitivity":"normal"}'::jsonb,
    'active',
    '{"type":"object","properties":{"question":{"type":"string"},"dataset_name":{"type":"string"},"dataset_ids":{"type":"array","items":{"type":"string"}}}}'::jsonb,
    '{"type":"object","properties":{"operation":{"type":"string"},"dataset_name":{"type":"string"}}}'::jsonb,
    '["datasets", "governance", "agents"]'::jsonb
),
(
    '01969888-3550-70c0-9000-000000000034',
    'Pipeline Operator',
    'Plans pipeline inspection, incremental runs and rebuilds with lineage-aware recommendations.',
    'operations',
    'native_pipeline',
    '{"default_pipeline_name":"daily_provider_build","input_datasets":["provider_events"],"output_datasets":["provider_metrics"],"sensitivity":"normal"}'::jsonb,
    'active',
    '{"type":"object","properties":{"question":{"type":"string"},"pipeline_name":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"run_mode":{"type":"string"},"pipeline_name":{"type":"string"}}}'::jsonb,
    '["pipelines", "lineage", "agents"]'::jsonb
),
(
    '01969888-3550-70c0-9000-000000000035',
    'Report Dispatcher',
    'Plans report generation, scheduling and governed multi-channel delivery.',
    'delivery',
    'native_report',
    '{"default_report_name":"ops_digest","default_channels":["email","slack","s3"],"sensitivity":"normal"}'::jsonb,
    'active',
    '{"type":"object","properties":{"question":{"type":"string"},"report_name":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"distribution_channels":{"type":"array"}}}'::jsonb,
    '["reports", "delivery", "agents"]'::jsonb
),
(
    '01969888-3550-70c0-9000-000000000036',
    'Workflow Operator',
    'Prepares approval-backed workflow proposals and submit_action orchestration.',
    'automation',
    'native_workflow',
    '{"requires_approval":true,"approval_scope":"operator","sensitivity":"mutating","default_workflow_name":"operator_review"}'::jsonb,
    'active',
    '{"type":"object","properties":{"question":{"type":"string"},"workflow_name":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"proposal_type":{"type":"string"}}}'::jsonb,
    '["workflows", "approvals", "agents"]'::jsonb
),
(
    '01969888-3550-70c0-9000-000000000037',
    'Code Repo Operator',
    'Prepares governed code-repo changes, branch plans and merge-request packaging.',
    'developer',
    'native_code_repo',
    '{"requires_approval":true,"approval_scope":"maintainer","sensitivity":"mutating","default_repository":"openfoundry-platform","branch_prefix":"agent","required_checks":["ci","policy","security"]}'::jsonb,
    'active',
    '{"type":"object","properties":{"question":{"type":"string"},"repository":{"type":"string"}}}'::jsonb,
    '{"type":"object","properties":{"branch":{"type":"string"},"merge_request_title":{"type":"string"}}}'::jsonb,
    '["developer", "git", "agents"]'::jsonb
)
ON CONFLICT (id) DO NOTHING;

UPDATE ai_agents
SET tool_ids = '[
    "01967f76-3550-70c0-9000-000000000030",
    "01967f76-3550-70c0-9000-000000000031",
    "01968522-3550-70c0-9000-000000000032",
    "01969888-3550-70c0-9000-000000000033",
    "01969888-3550-70c0-9000-000000000034",
    "01969888-3550-70c0-9000-000000000035",
    "01969888-3550-70c0-9000-000000000036",
    "01969888-3550-70c0-9000-000000000037"
]'::jsonb,
    updated_at = NOW()
WHERE id = '01967f76-3550-70c0-9000-000000000040';
