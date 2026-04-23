CREATE TABLE IF NOT EXISTS streaming_streams (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    schema JSONB NOT NULL,
    source_binding JSONB NOT NULL,
    retention_hours INTEGER NOT NULL DEFAULT 72,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS streaming_windows (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    window_type TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL,
    slide_seconds INTEGER NOT NULL,
    session_gap_seconds INTEGER NOT NULL,
    allowed_lateness_seconds INTEGER NOT NULL,
    aggregation_keys JSONB NOT NULL,
    measure_fields JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS streaming_topologies (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    nodes JSONB NOT NULL,
    edges JSONB NOT NULL,
    join_definition JSONB,
    cep_definition JSONB,
    backpressure_policy JSONB NOT NULL,
    source_stream_ids JSONB NOT NULL,
    sink_bindings JSONB NOT NULL,
    state_backend TEXT NOT NULL DEFAULT 'rocksdb',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS streaming_topology_runs (
    id UUID PRIMARY KEY,
    topology_id UUID NOT NULL REFERENCES streaming_topologies(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    metrics JSONB NOT NULL,
    aggregate_windows JSONB NOT NULL,
    live_tail JSONB NOT NULL,
    cep_matches JSONB NOT NULL,
    state_snapshot JSONB NOT NULL,
    backpressure_snapshot JSONB NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_streaming_topologies_status
    ON streaming_topologies(status);

CREATE INDEX IF NOT EXISTS idx_streaming_runs_topology_id
    ON streaming_topology_runs(topology_id, created_at DESC);

INSERT INTO streaming_streams (id, name, description, status, schema, source_binding, retention_hours)
VALUES
(
    '01968040-0850-7920-9000-000000000001',
    'Orders Ingress',
    'Kafka topic carrying customer checkout events.',
    'active',
    '{
      "fields": [
        {"name": "event_time", "data_type": "timestamp", "nullable": false, "semantic_role": "event_time"},
        {"name": "customer_id", "data_type": "string", "nullable": false, "semantic_role": "join_key"},
        {"name": "amount", "data_type": "double", "nullable": false, "semantic_role": "metric"}
      ],
      "primary_key": null,
      "watermark_field": "event_time"
    }'::jsonb,
    '{
      "connector_type": "kafka",
      "endpoint": "kafka://stream/orders",
      "format": "json",
      "config": {"compression": "snappy", "topic": "orders.v1"}
    }'::jsonb,
    72
),
(
    '01968040-0850-7920-9000-000000000002',
    'Payments Feed',
    'NATS subject with payment lifecycle updates.',
    'active',
    '{
      "fields": [
        {"name": "event_time", "data_type": "timestamp", "nullable": false, "semantic_role": "event_time"},
        {"name": "customer_id", "data_type": "string", "nullable": false, "semantic_role": "join_key"},
        {"name": "status", "data_type": "string", "nullable": false, "semantic_role": "state"}
      ],
      "primary_key": null,
      "watermark_field": "event_time"
    }'::jsonb,
    '{
      "connector_type": "nats",
      "endpoint": "nats://payments.authorized",
      "format": "json",
      "config": {"durable_consumer": "payments-materializer"}
    }'::jsonb,
    48
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO streaming_windows (
    id, name, description, status, window_type, duration_seconds, slide_seconds,
    session_gap_seconds, allowed_lateness_seconds, aggregation_keys, measure_fields
)
VALUES (
    '01968040-0850-7920-9000-000000000010',
    'Five Minute Revenue',
    'Tumbling revenue aggregates keyed by customer and risk band.',
    'active',
    'tumbling',
    300,
    300,
    180,
    30,
    '["customer_id", "risk_band"]'::jsonb,
    '["amount"]'::jsonb
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO streaming_topologies (
    id, name, description, status, nodes, edges, join_definition, cep_definition,
    backpressure_policy, source_stream_ids, sink_bindings, state_backend
)
VALUES (
    '01968040-0850-7920-9000-000000000020',
    'Revenue Anomaly Pipeline',
    'Join checkout and payment events, compute windowed aggregates, and push live anomalies.',
    'active',
    '[
      {"id": "src-orders", "label": "Orders", "node_type": "source", "stream_id": "01968040-0850-7920-9000-000000000001", "window_id": null, "config": {"parallelism": 3}},
      {"id": "src-payments", "label": "Payments", "node_type": "source", "stream_id": "01968040-0850-7920-9000-000000000002", "window_id": null, "config": {"parallelism": 2}},
      {"id": "join-risk", "label": "Join", "node_type": "join", "stream_id": null, "window_id": null, "config": {"type": "stream-stream"}},
      {"id": "window-revenue", "label": "Five Minute Window", "node_type": "window", "stream_id": null, "window_id": "01968040-0850-7920-9000-000000000010", "config": {"emit": "incremental"}},
      {"id": "sink-live", "label": "Live Tail", "node_type": "sink", "stream_id": null, "window_id": null, "config": {"connector": "websocket"}}
    ]'::jsonb,
    '[
      {"source_node_id": "src-orders", "target_node_id": "join-risk", "label": "orders"},
      {"source_node_id": "src-payments", "target_node_id": "join-risk", "label": "payments"},
      {"source_node_id": "join-risk", "target_node_id": "window-revenue", "label": "enriched-events"},
      {"source_node_id": "window-revenue", "target_node_id": "sink-live", "label": "alerts"}
    ]'::jsonb,
    '{
      "join_type": "stream-stream",
      "left_stream_id": "01968040-0850-7920-9000-000000000001",
      "right_stream_id": "01968040-0850-7920-9000-000000000002",
      "table_name": "payments_lookup",
      "key_fields": ["customer_id"],
      "window_seconds": 600
    }'::jsonb,
    '{
      "pattern_name": "payment-before-order",
      "sequence": ["authorized", "captured", "checkout"],
      "within_seconds": 900,
      "output_stream": "fraud_alerts"
    }'::jsonb,
    '{
      "max_in_flight": 512,
      "queue_capacity": 2048,
      "throttle_strategy": "credit-based"
    }'::jsonb,
    '["01968040-0850-7920-9000-000000000001", "01968040-0850-7920-9000-000000000002"]'::jsonb,
    '[
      {"connector_type": "websocket", "endpoint": "ws://localhost:8080/api/v1/streaming/live-tail", "format": "json", "config": {"channel": "revenue-alerts"}},
      {"connector_type": "dataset", "endpoint": "dataset://materialized/revenue_alerts", "format": "parquet", "config": {"mode": "append"}}
    ]'::jsonb,
    'rocksdb'
)
ON CONFLICT (id) DO NOTHING;