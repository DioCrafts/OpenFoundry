UPDATE ai_tools
SET execution_mode = 'native_sql',
    execution_config = jsonb_build_object(
        'default_dataset_name', 'provider_metrics',
        'time_column', 'event_date',
        'default_limit', 100,
        'metric_hints', jsonb_build_array('avg_latency_ms', 'error_rate')
    ),
    updated_at = NOW()
WHERE id = '01967f76-3550-70c0-9000-000000000030';

UPDATE ai_tools
SET execution_mode = 'native_ontology',
    execution_config = jsonb_build_object(
        'default_object_types', jsonb_build_array('Incident', 'Provider'),
        'default_link_type', 'AFFECTS'
    ),
    updated_at = NOW()
WHERE id = '01967f76-3550-70c0-9000-000000000031';

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
) VALUES (
    '01968522-3550-70c0-9000-000000000032',
    'Knowledge Search',
    'Re-ranks retrieved passages and returns grounded citations for the current agent request.',
    'retrieval',
    'knowledge_search',
    '{"top_k":4,"min_score":0.15}'::jsonb,
    'active',
    '{"type":"object","properties":{"query":{"type":"string"},"top_k":{"type":"integer"}}}'::jsonb,
    '{"type":"object","properties":{"results":{"type":"array"}}}'::jsonb,
    '["rag", "retrieval", "agents"]'::jsonb
) ON CONFLICT (id) DO NOTHING;

UPDATE ai_agents
SET tool_ids = '[
    "01967f76-3550-70c0-9000-000000000030",
    "01967f76-3550-70c0-9000-000000000031",
    "01968522-3550-70c0-9000-000000000032"
]'::jsonb,
    updated_at = NOW()
WHERE id = '01967f76-3550-70c0-9000-000000000040';
