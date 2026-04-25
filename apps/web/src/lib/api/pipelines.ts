import api from './client';

export interface PipelineScheduleConfig {
  enabled: boolean;
  cron: string | null;
}

export interface PipelineRetryPolicy {
  max_attempts: number;
  retry_on_failure: boolean;
  allow_partial_reexecution: boolean;
}

export interface PipelineColumnMapping {
  source_dataset_id: string | null;
  source_column: string;
  target_column: string;
}

export interface PipelineNode {
  id: string;
  label: string;
  transform_type: string;
  config: Record<string, unknown>;
  depends_on: string[];
  input_dataset_ids: string[];
  output_dataset_id: string | null;
}

export interface Pipeline {
  id: string;
  name: string;
  description: string;
  owner_id: string;
  dag: PipelineNode[];
  status: string;
  schedule_config: PipelineScheduleConfig;
  retry_policy: PipelineRetryPolicy;
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface PipelineNodeResult {
  node_id: string;
  label: string;
  transform_type: string;
  status: string;
  rows_affected: number | null;
  attempts: number;
  output: Record<string, unknown> | null;
  error: string | null;
}

export interface PipelineRun {
  id: string;
  pipeline_id: string;
  status: string;
  trigger_type: string;
  started_by: string | null;
  attempt_number: number;
  started_from_node_id: string | null;
  retry_of_run_id: string | null;
  execution_context: Record<string, unknown>;
  node_results: PipelineNodeResult[] | null;
  error_message: string | null;
  started_at: string;
  finished_at: string | null;
}

export interface LineageNode {
  id: string;
  kind: string;
  label: string;
  marking: string;
  metadata: Record<string, unknown>;
}

export interface LineageEdge {
  id: string;
  source: string;
  source_kind: string;
  target: string;
  target_kind: string;
  relation_kind: string;
  pipeline_id: string | null;
  workflow_id: string | null;
  node_id: string | null;
  step_id: string | null;
  effective_marking: string;
  metadata: Record<string, unknown>;
}

export interface LineageGraph {
  nodes: LineageNode[];
  edges: LineageEdge[];
}

export interface LineagePathHop {
  source_id: string;
  source_kind: string;
  target_id: string;
  target_kind: string;
  relation_kind: string;
  effective_marking: string;
}

export interface LineageImpactItem {
  id: string;
  kind: string;
  label: string;
  distance: number;
  marking: string;
  effective_marking: string;
  requires_acknowledgement: boolean;
  metadata: Record<string, unknown>;
  path: LineagePathHop[];
}

export interface LineageBuildCandidate {
  id: string;
  kind: string;
  label: string;
  status: string | null;
  distance: number;
  triggerable: boolean;
  marking: string;
  effective_marking: string;
  requires_acknowledgement: boolean;
  blocked_reason: string | null;
  metadata: Record<string, unknown>;
}

export interface LineageImpactAnalysis {
  root: LineageNode;
  propagated_marking: string;
  upstream: LineageImpactItem[];
  downstream: LineageImpactItem[];
  build_candidates: LineageBuildCandidate[];
}

export interface LineageBuildTriggerResult {
  id: string;
  kind: string;
  label: string;
  run_id: string | null;
  status: string;
  message: string | null;
}

export interface LineageBuildResult {
  root: LineageNode;
  dry_run: boolean;
  acknowledged_sensitive_lineage: boolean;
  propagated_marking: string;
  candidates: LineageBuildCandidate[];
  triggered: LineageBuildTriggerResult[];
  skipped: LineageBuildTriggerResult[];
}

export interface ColumnLineageEdge {
  id: string;
  source_dataset_id: string;
  source_column: string;
  target_dataset_id: string;
  target_column: string;
  pipeline_id: string | null;
  node_id: string | null;
  created_at: string;
}

// Pipeline CRUD
export function listPipelines(params?: { page?: number; per_page?: number; search?: string; status?: string }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  if (params?.status) qs.set('status', params.status);
  return api.get<{ data: Pipeline[]; total: number; page: number; per_page: number }>(
    `/pipelines?${qs}`,
  );
}

export function getPipeline(id: string) {
  return api.get<Pipeline>(`/pipelines/${id}`);
}

export function createPipeline(body: {
  name: string;
  description?: string;
  status?: string;
  nodes: PipelineNode[];
  schedule_config?: PipelineScheduleConfig;
  retry_policy?: PipelineRetryPolicy;
}) {
  return api.post<Pipeline>('/pipelines', body);
}

export function updatePipeline(id: string, body: {
  name?: string;
  description?: string;
  status?: string;
  nodes?: PipelineNode[];
  schedule_config?: PipelineScheduleConfig;
  retry_policy?: PipelineRetryPolicy;
}) {
  return api.put<Pipeline>(`/pipelines/${id}`, body);
}

export function deletePipeline(id: string) {
  return api.delete(`/pipelines/${id}`);
}

// Execution
export function triggerRun(pipelineId: string, body?: { from_node_id?: string; context?: Record<string, unknown>; skip_unchanged?: boolean }) {
  return api.post<PipelineRun>(`/pipelines/${pipelineId}/run`, body ?? {});
}

export function listRuns(pipelineId: string, params?: { page?: number; per_page?: number }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: PipelineRun[] }>(`/pipelines/${pipelineId}/runs?${qs}`);
}

export function retryPipelineRun(pipelineId: string, runId: string, body?: { from_node_id?: string; skip_unchanged?: boolean }) {
  return api.post<PipelineRun>(`/pipelines/${pipelineId}/runs/${runId}/retry`, body ?? {});
}

export function runDuePipelines() {
  return api.post<{ triggered_runs: number }>('/pipelines/triggers/cron/run-due', {});
}

// Lineage
export function getDatasetLineage(datasetId: string) {
  return api.get<LineageGraph>(`/lineage/datasets/${datasetId}`);
}

export function getDatasetColumnLineage(datasetId: string) {
  return api.get<ColumnLineageEdge[]>(`/lineage/datasets/${datasetId}/columns`);
}

export function getDatasetLineageImpact(datasetId: string) {
  return api.get<LineageImpactAnalysis>(`/lineage/datasets/${datasetId}/impact`);
}

export function triggerLineageBuilds(datasetId: string, body?: {
  include_workflows?: boolean;
  dry_run?: boolean;
  acknowledge_sensitive_lineage?: boolean;
  max_depth?: number;
  context?: Record<string, unknown>;
}) {
  return api.post<LineageBuildResult>(`/lineage/datasets/${datasetId}/builds`, body ?? {});
}

export function getFullLineage() {
  return api.get<LineageGraph>('/lineage');
}
