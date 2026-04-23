CREATE TABLE IF NOT EXISTS ml_experiments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    objective TEXT NOT NULL DEFAULT '',
    task_type TEXT NOT NULL DEFAULT 'classification',
    primary_metric TEXT NOT NULL DEFAULT 'accuracy',
    status TEXT NOT NULL DEFAULT 'active',
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    owner_id UUID NULL,
    run_count BIGINT NOT NULL DEFAULT 0,
    best_metric JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_runs (
    id UUID PRIMARY KEY,
    experiment_id UUID NOT NULL REFERENCES ml_experiments(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'completed',
    params JSONB NOT NULL DEFAULT '{}'::jsonb,
    metrics JSONB NOT NULL DEFAULT '[]'::jsonb,
    artifacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    notes TEXT NOT NULL DEFAULT '',
    source_dataset_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    model_version_id UUID NULL,
    started_at TIMESTAMPTZ NULL,
    finished_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_models (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    problem_type TEXT NOT NULL DEFAULT 'classification',
    status TEXT NOT NULL DEFAULT 'active',
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    owner_id UUID NULL,
    current_stage TEXT NOT NULL DEFAULT 'none',
    latest_version_number INTEGER NULL,
    active_deployment_id UUID NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_model_versions (
    id UUID PRIMARY KEY,
    model_id UUID NOT NULL REFERENCES ml_models(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    version_label TEXT NOT NULL,
    stage TEXT NOT NULL DEFAULT 'candidate',
    source_run_id UUID NULL,
    training_job_id UUID NULL,
    hyperparameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    metrics JSONB NOT NULL DEFAULT '[]'::jsonb,
    artifact_uri TEXT NULL,
    schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    promoted_at TIMESTAMPTZ NULL,
    UNIQUE(model_id, version_number)
);

CREATE TABLE IF NOT EXISTS ml_features (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    entity_name TEXT NOT NULL,
    data_type TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    offline_source TEXT NOT NULL DEFAULT '',
    transformation TEXT NOT NULL DEFAULT '',
    online_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    online_namespace TEXT NOT NULL DEFAULT '',
    batch_schedule TEXT NOT NULL DEFAULT '0 * * * *',
    freshness_sla_minutes INTEGER NOT NULL DEFAULT 60,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    samples JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_materialized_at TIMESTAMPTZ NULL,
    last_online_sync_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_training_jobs (
    id UUID PRIMARY KEY,
    experiment_id UUID NULL REFERENCES ml_experiments(id) ON DELETE SET NULL,
    model_id UUID NULL REFERENCES ml_models(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    dataset_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    training_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    hyperparameter_search JSONB NOT NULL DEFAULT '{}'::jsonb,
    objective_metric_name TEXT NOT NULL DEFAULT 'accuracy',
    trials JSONB NOT NULL DEFAULT '[]'::jsonb,
    best_model_version_id UUID NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_deployments (
    id UUID PRIMARY KEY,
    model_id UUID NOT NULL REFERENCES ml_models(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    strategy_type TEXT NOT NULL DEFAULT 'single',
    endpoint_path TEXT NOT NULL UNIQUE,
    traffic_split JSONB NOT NULL DEFAULT '[]'::jsonb,
    monitoring_window TEXT NOT NULL DEFAULT '24h',
    baseline_dataset_id UUID NULL,
    drift_report JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ml_batch_predictions (
    id UUID PRIMARY KEY,
    deployment_id UUID NOT NULL REFERENCES ml_deployments(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'completed',
    record_count BIGINT NOT NULL DEFAULT 0,
    output_destination TEXT NULL,
    outputs JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_ml_runs_experiment_id ON ml_runs(experiment_id);
CREATE INDEX IF NOT EXISTS idx_ml_model_versions_model_id ON ml_model_versions(model_id);
CREATE INDEX IF NOT EXISTS idx_ml_training_jobs_model_id ON ml_training_jobs(model_id);
CREATE INDEX IF NOT EXISTS idx_ml_training_jobs_experiment_id ON ml_training_jobs(experiment_id);
CREATE INDEX IF NOT EXISTS idx_ml_deployments_model_id ON ml_deployments(model_id);
CREATE INDEX IF NOT EXISTS idx_ml_batch_predictions_deployment_id ON ml_batch_predictions(deployment_id);

INSERT INTO ml_experiments (
    id,
    name,
    description,
    objective,
    task_type,
    primary_metric,
    status,
    tags,
    run_count,
    best_metric
) VALUES (
    '01967f76-3550-70c0-8000-000000000001',
    'Churn Prevention',
    'Weekly churn training across account health, usage, and support signals.',
    'Reduce churn risk for strategic accounts before renewal.',
    'classification',
    'f1',
    'active',
    '["retention", "b2b"]'::jsonb,
    1,
    '{"name":"f1","value":0.84}'::jsonb
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_runs (
    id,
    experiment_id,
    name,
    status,
    params,
    metrics,
    artifacts,
    notes,
    source_dataset_ids,
    started_at,
    finished_at
) VALUES (
    '01967f76-3550-70c0-8000-000000000002',
    '01967f76-3550-70c0-8000-000000000001',
    'weekly-xgboost-2026-04-22',
    'completed',
    '{"learning_rate":0.05,"max_depth":6,"subsample":0.9}'::jsonb,
    '[{"name":"accuracy","value":0.81},{"name":"f1","value":0.84},{"name":"auc","value":0.88}]'::jsonb,
    '[{"id":"01967f76-3550-70c0-8000-000000000003","name":"feature-importance.json","uri":"ml://artifacts/churn/importance.json","artifact_type":"json","size_bytes":3124}]'::jsonb,
    'Champion training run used for the April rollout.',
    '[]'::jsonb,
    NOW() - INTERVAL '2 days',
    NOW() - INTERVAL '2 days' + INTERVAL '14 minutes'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_models (
    id,
    name,
    description,
    problem_type,
    status,
    tags,
    current_stage,
    latest_version_number
) VALUES (
    '01967f76-3550-70c0-8000-000000000010',
    'Account Churn Classifier',
    'Classifier used by renewal operations to identify at-risk accounts.',
    'classification',
    'active',
    '["xgboost", "production"]'::jsonb,
    'production',
    2
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_model_versions (
    id,
    model_id,
    version_number,
    version_label,
    stage,
    source_run_id,
    hyperparameters,
    metrics,
    artifact_uri,
    schema,
    promoted_at
) VALUES
(
    '01967f76-3550-70c0-8000-000000000011',
    '01967f76-3550-70c0-8000-000000000010',
    1,
    'v1-baseline',
    'staging',
    '01967f76-3550-70c0-8000-000000000002',
    '{"learning_rate":0.03,"max_depth":5}'::jsonb,
    '[{"name":"f1","value":0.79}]'::jsonb,
    'ml://models/churn/v1',
    '{"signature":"tabular-binary"}'::jsonb,
    NOW() - INTERVAL '9 days'
),
(
    '01967f76-3550-70c0-8000-000000000012',
    '01967f76-3550-70c0-8000-000000000010',
    2,
    'v2-champion',
    'production',
    '01967f76-3550-70c0-8000-000000000002',
    '{"learning_rate":0.05,"max_depth":6}'::jsonb,
    '[{"name":"f1","value":0.84}]'::jsonb,
    'ml://models/churn/v2',
    '{"signature":"tabular-binary"}'::jsonb,
    NOW() - INTERVAL '2 days'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_features (
    id,
    name,
    entity_name,
    data_type,
    description,
    offline_source,
    transformation,
    online_enabled,
    online_namespace,
    batch_schedule,
    freshness_sla_minutes,
    tags,
    samples,
    last_materialized_at,
    last_online_sync_at
) VALUES (
    '01967f76-3550-70c0-8000-000000000020',
    'avg_ticket_resolution_hours_30d',
    'account',
    'float',
    'Rolling 30-day average ticket resolution time by account.',
    'SELECT account_id, AVG(resolution_hours) AS value FROM support_tickets GROUP BY account_id',
    'coalesce(avg(resolution_hours), 0)',
    TRUE,
    'ml:features:account',
    '0 * * * *',
    60,
    '["support", "freshness-critical"]'::jsonb,
    '[{"entity_key":"acct_001","value":12.4},{"entity_key":"acct_002","value":6.8}]'::jsonb,
    NOW() - INTERVAL '1 hour',
    NOW() - INTERVAL '1 hour'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_training_jobs (
    id,
    experiment_id,
    model_id,
    name,
    status,
    dataset_ids,
    training_config,
    hyperparameter_search,
    objective_metric_name,
    trials,
    best_model_version_id,
    submitted_at,
    started_at,
    completed_at
) VALUES (
    '01967f76-3550-70c0-8000-000000000030',
    '01967f76-3550-70c0-8000-000000000001',
    '01967f76-3550-70c0-8000-000000000010',
    'nightly-retention-tuning',
    'completed',
    '[]'::jsonb,
    '{"engine":"xgboost","workers":3}'::jsonb,
    '{"method":"random-search"}'::jsonb,
    'f1',
    '[{"id":"trial-1","status":"completed","hyperparameters":{"learning_rate":0.03},"objective_metric":{"name":"f1","value":0.79}},{"id":"trial-2","status":"completed","hyperparameters":{"learning_rate":0.05},"objective_metric":{"name":"f1","value":0.84}}]'::jsonb,
    '01967f76-3550-70c0-8000-000000000012',
    NOW() - INTERVAL '2 days',
    NOW() - INTERVAL '2 days',
    NOW() - INTERVAL '2 days' + INTERVAL '18 minutes'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO ml_deployments (
    id,
    model_id,
    name,
    status,
    strategy_type,
    endpoint_path,
    traffic_split,
    monitoring_window,
    drift_report
) VALUES (
    '01967f76-3550-70c0-8000-000000000040',
    '01967f76-3550-70c0-8000-000000000010',
    'renewal-risk-service',
    'active',
    'ab_test',
    '/api/v1/ml/runtime/renewal-risk',
    '[{"model_version_id":"01967f76-3550-70c0-8000-000000000012","label":"champion","allocation":85},{"model_version_id":"01967f76-3550-70c0-8000-000000000011","label":"challenger","allocation":15}]'::jsonb,
    '24h',
    '{"generated_at":"2026-04-22T10:15:00Z","dataset_metrics":[{"name":"psi","score":0.21,"threshold":0.25,"status":"warning"}],"concept_metrics":[{"name":"prediction_target_gap","score":0.14,"threshold":0.18,"status":"healthy"}],"recommend_retraining":false,"auto_retraining_job_id":null,"notes":"Current drift is elevated but still below the retraining threshold."}'::jsonb
) ON CONFLICT (id) DO NOTHING;

UPDATE ml_models
SET active_deployment_id = '01967f76-3550-70c0-8000-000000000040'
WHERE id = '01967f76-3550-70c0-8000-000000000010';

INSERT INTO ml_batch_predictions (
    id,
    deployment_id,
    status,
    record_count,
    output_destination,
    outputs,
    created_at,
    completed_at
) VALUES (
    '01967f76-3550-70c0-8000-000000000050',
    '01967f76-3550-70c0-8000-000000000040',
    'completed',
    2,
    's3://open-foundry-ml/predictions/churn/2026-04-22.parquet',
    '[{"record_id":"record-1","variant":"champion","model_version_id":"01967f76-3550-70c0-8000-000000000012","predicted_label":"positive","score":0.78,"confidence":0.85,"contributions":[{"name":"usage_decline","value":0.42}]},{"record_id":"record-2","variant":"challenger","model_version_id":"01967f76-3550-70c0-8000-000000000011","predicted_label":"negative","score":0.31,"confidence":0.73,"contributions":[{"name":"ticket_volume","value":0.28}]}]'::jsonb,
    NOW() - INTERVAL '8 hours',
    NOW() - INTERVAL '8 hours' + INTERVAL '3 minutes'
) ON CONFLICT (id) DO NOTHING;