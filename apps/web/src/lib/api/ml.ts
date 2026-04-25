import api from './client';

export interface MlStudioOverview {
  experiment_count: number;
  active_run_count: number;
  model_count: number;
  production_model_count: number;
  feature_count: number;
  online_feature_count: number;
  deployment_count: number;
  ab_test_count: number;
  drift_alert_count: number;
  queued_training_jobs: number;
}

export interface MetricValue {
  name: string;
  value: number;
}

export interface ArtifactReference {
  id: string;
  name: string;
  uri: string;
  artifact_type: string;
  size_bytes: number;
}

export interface Experiment {
  id: string;
  name: string;
  description: string;
  objective: string;
  objective_spec: {
    status: string;
    deployment_target: string;
    stakeholders: string[];
    success_criteria: string[];
    linked_dataset_ids: string[];
    linked_model_ids: string[];
    documentation_uri: string;
    collaboration_notes: string[];
  };
  task_type: string;
  primary_metric: string;
  status: string;
  tags: string[];
  run_count: number;
  best_metric: MetricValue | null;
  owner_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ExperimentRun {
  id: string;
  experiment_id: string;
  name: string;
  status: string;
  params: unknown;
  metrics: MetricValue[];
  artifacts: ArtifactReference[];
  notes: string;
  source_dataset_ids: string[];
  model_version_id: string | null;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CompareRunsResponse {
  data: ExperimentRun[];
  metric_names: string[];
}

export interface RegisteredModel {
  id: string;
  name: string;
  description: string;
  problem_type: string;
  status: string;
  tags: string[];
  owner_id: string | null;
  current_stage: string;
  latest_version_number: number | null;
  active_deployment_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface ModelVersion {
  id: string;
  model_id: string;
  version_number: number;
  version_label: string;
  stage: string;
  source_run_id: string | null;
  training_job_id: string | null;
  hyperparameters: unknown;
  metrics: MetricValue[];
  artifact_uri: string | null;
  schema: unknown;
  created_at: string;
  promoted_at: string | null;
}

export interface FeatureSample {
  entity_key: string;
  value: unknown;
  observed_at?: string | null;
}

export interface FeatureDefinition {
  id: string;
  name: string;
  entity_name: string;
  data_type: string;
  description: string;
  status: string;
  offline_source: string;
  transformation: string;
  online_enabled: boolean;
  online_namespace: string;
  batch_schedule: string;
  freshness_sla_minutes: number;
  tags: string[];
  samples: FeatureSample[];
  last_materialized_at: string | null;
  last_online_sync_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface OnlineFeatureSnapshot {
  feature_id: string;
  namespace: string;
  source: string;
  values: FeatureSample[];
  fetched_at: string;
}

export interface TrainingTrial {
  id: string;
  status: string;
  hyperparameters: unknown;
  objective_metric: MetricValue;
}

export interface TrainingJob {
  id: string;
  experiment_id: string | null;
  model_id: string | null;
  name: string;
  status: string;
  dataset_ids: string[];
  training_config: unknown;
  hyperparameter_search: unknown;
  objective_metric_name: string;
  trials: TrainingTrial[];
  best_model_version_id: string | null;
  submitted_at: string;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface TrafficSplitEntry {
  model_version_id: string;
  label: string;
  allocation: number;
}

export interface DriftMetric {
  name: string;
  score: number;
  threshold: number;
  status: string;
}

export interface DriftReport {
  generated_at: string;
  dataset_metrics: DriftMetric[];
  concept_metrics: DriftMetric[];
  recommend_retraining: boolean;
  auto_retraining_job_id: string | null;
  notes: string;
}

export interface ModelDeployment {
  id: string;
  model_id: string;
  name: string;
  status: string;
  strategy_type: string;
  endpoint_path: string;
  traffic_split: TrafficSplitEntry[];
  monitoring_window: string;
  baseline_dataset_id: string | null;
  drift_report: DriftReport | null;
  created_at: string;
  updated_at: string;
}

export interface FeatureContribution {
  name: string;
  value: number;
}

export interface PredictionOutput {
  record_id: string;
  variant: string;
  model_version_id: string;
  predicted_label: string;
  score: number;
  confidence: number;
  contributions: FeatureContribution[];
}

export interface RealtimePredictionResponse {
  deployment_id: string;
  outputs: PredictionOutput[];
  predicted_at: string;
}

export interface BatchPredictionJob {
  id: string;
  deployment_id: string;
  status: string;
  record_count: number;
  output_destination: string | null;
  outputs: PredictionOutput[];
  created_at: string;
  completed_at: string | null;
}

export interface ListResponse<T> {
  data: T[];
}

export interface ExperimentAssetLineageResponse {
  experiment_id: string;
  objective_status: string;
  nodes: Array<{
    id: string;
    kind: string;
    label: string;
    status: string;
    metadata: Record<string, unknown>;
  }>;
  edges: Array<{
    source: string;
    target: string;
    relation: string;
  }>;
  summary: {
    dataset_count: number;
    run_count: number;
    training_job_count: number;
    model_count: number;
    version_count: number;
    deployment_count: number;
    frameworks: string[];
  };
}

export function getOverview() {
  return api.get<MlStudioOverview>('/ml/overview');
}

export function listExperiments() {
  return api.get<ListResponse<Experiment>>('/ml/experiments');
}

export function createExperiment(body: {
  name: string;
  description?: string;
  objective?: string;
  objective_spec?: {
    status: string;
    deployment_target: string;
    stakeholders: string[];
    success_criteria: string[];
    linked_dataset_ids: string[];
    linked_model_ids: string[];
    documentation_uri: string;
    collaboration_notes: string[];
  };
  task_type?: string;
  primary_metric?: string;
  tags?: string[];
}) {
  return api.post<Experiment>('/ml/experiments', body);
}

export function updateExperiment(id: string, body: {
  name?: string;
  description?: string;
  objective?: string;
  objective_spec?: {
    status: string;
    deployment_target: string;
    stakeholders: string[];
    success_criteria: string[];
    linked_dataset_ids: string[];
    linked_model_ids: string[];
    documentation_uri: string;
    collaboration_notes: string[];
  };
  task_type?: string;
  primary_metric?: string;
  status?: string;
  tags?: string[];
}) {
  return api.patch<Experiment>(`/ml/experiments/${id}`, body);
}

export function getExperimentAssetLineage(id: string) {
  return api.get<ExperimentAssetLineageResponse>(`/ml/experiments/${id}/asset-lineage`);
}

export function listRuns(experimentId: string) {
  return api.get<ListResponse<ExperimentRun>>(`/ml/experiments/${experimentId}/runs`);
}

export function createRun(experimentId: string, body: {
  name: string;
  status?: string;
  params?: unknown;
  metrics?: MetricValue[];
  artifacts?: ArtifactReference[];
  notes?: string;
  source_dataset_ids?: string[];
  started_at?: string | null;
  finished_at?: string | null;
}) {
  return api.post<ExperimentRun>(`/ml/experiments/${experimentId}/runs`, body);
}

export function compareRuns(runIds: string[]) {
  return api.post<CompareRunsResponse>('/ml/runs/compare', { run_ids: runIds });
}

export function listModels() {
  return api.get<ListResponse<RegisteredModel>>('/ml/models');
}

export function createModel(body: {
  name: string;
  description?: string;
  problem_type?: string;
  status?: string;
  tags?: string[];
}) {
  return api.post<RegisteredModel>('/ml/models', body);
}

export function updateModel(id: string, body: {
  name?: string;
  description?: string;
  problem_type?: string;
  status?: string;
  tags?: string[];
}) {
  return api.patch<RegisteredModel>(`/ml/models/${id}`, body);
}

export function listModelVersions(modelId: string) {
  return api.get<ListResponse<ModelVersion>>(`/ml/models/${modelId}/versions`);
}

export function createModelVersion(modelId: string, body: {
  version_label?: string;
  stage?: string;
  source_run_id?: string | null;
  training_job_id?: string | null;
  hyperparameters?: unknown;
  metrics?: MetricValue[];
  artifact_uri?: string | null;
  schema?: unknown;
}) {
  return api.post<ModelVersion>(`/ml/models/${modelId}/versions`, body);
}

export function transitionModelVersion(versionId: string, stage: string) {
  return api.post<ModelVersion>(`/ml/model-versions/${versionId}/transition`, { stage });
}

export function listFeatures() {
  return api.get<ListResponse<FeatureDefinition>>('/ml/features');
}

export function createFeature(body: {
  name: string;
  entity_name: string;
  data_type: string;
  description?: string;
  offline_source?: string;
  transformation?: string;
  online_enabled?: boolean;
  online_namespace?: string;
  batch_schedule?: string;
  freshness_sla_minutes?: number;
  tags?: string[];
  samples?: FeatureSample[];
}) {
  return api.post<FeatureDefinition>('/ml/features', body);
}

export function updateFeature(id: string, body: {
  name?: string;
  entity_name?: string;
  data_type?: string;
  description?: string;
  status?: string;
  offline_source?: string;
  transformation?: string;
  online_enabled?: boolean;
  online_namespace?: string;
  batch_schedule?: string;
  freshness_sla_minutes?: number;
  tags?: string[];
}) {
  return api.patch<FeatureDefinition>(`/ml/features/${id}`, body);
}

export function materializeFeature(id: string, body: {
  samples?: FeatureSample[];
  mode?: string;
}) {
  return api.post<FeatureDefinition>(`/ml/features/${id}/materialize`, body);
}

export function getOnlineFeatureSnapshot(id: string) {
  return api.get<OnlineFeatureSnapshot>(`/ml/features/${id}/online`);
}

export function listTrainingJobs() {
  return api.get<ListResponse<TrainingJob>>('/ml/training-jobs');
}

export function createTrainingJob(body: {
  experiment_id?: string | null;
  model_id?: string | null;
  name: string;
  dataset_ids?: string[];
  training_config?: unknown;
  hyperparameter_search?: unknown;
  objective_metric_name?: string;
  auto_register_model_version?: boolean;
}) {
  return api.post<TrainingJob>('/ml/training-jobs', body);
}

export function listDeployments() {
  return api.get<ListResponse<ModelDeployment>>('/ml/deployments');
}

export function createDeployment(body: {
  model_id: string;
  name: string;
  strategy_type?: string;
  endpoint_path: string;
  traffic_split?: TrafficSplitEntry[];
  monitoring_window?: string;
  baseline_dataset_id?: string | null;
}) {
  return api.post<ModelDeployment>('/ml/deployments', body);
}

export function updateDeployment(id: string, body: {
  name?: string;
  status?: string;
  strategy_type?: string;
  endpoint_path?: string;
  traffic_split?: TrafficSplitEntry[];
  monitoring_window?: string;
  baseline_dataset_id?: string | null;
}) {
  return api.patch<ModelDeployment>(`/ml/deployments/${id}`, body);
}

export function generateDriftReport(id: string, body: {
  baseline_rows?: number;
  observed_rows?: number;
  auto_retrain?: boolean;
}) {
  return api.post<ModelDeployment>(`/ml/deployments/${id}/drift`, body);
}

export function realtimePredict(id: string, body: {
  inputs: unknown[];
  explain?: boolean;
}) {
  return api.post<RealtimePredictionResponse>(`/ml/deployments/${id}/predict`, body);
}

export function listBatchPredictions() {
  return api.get<ListResponse<BatchPredictionJob>>('/ml/batch-predictions');
}

export function createBatchPrediction(body: {
  deployment_id: string;
  records: unknown[];
  output_destination?: string | null;
}) {
  return api.post<BatchPredictionJob>('/ml/batch-predictions', body);
}
