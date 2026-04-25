import api from './client';

export interface WorkflowBranchCondition {
  field: string;
  operator: string;
  value: unknown;
}

export interface WorkflowBranch {
  condition: WorkflowBranchCondition;
  next_step_id: string;
}

export interface WorkflowStep {
  id: string;
  name: string;
  step_type: string;
  description: string;
  config: Record<string, unknown>;
  next_step_id: string | null;
  branches: WorkflowBranch[];
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
  owner_id: string;
  status: string;
  trigger_type: string;
  trigger_config: Record<string, unknown>;
  steps: WorkflowStep[];
  webhook_secret: string | null;
  next_run_at: string | null;
  last_triggered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface WorkflowRun {
  id: string;
  workflow_id: string;
  trigger_type: string;
  status: string;
  started_by: string | null;
  current_step_id: string | null;
  context: Record<string, unknown>;
  error_message: string | null;
  started_at: string;
  finished_at: string | null;
}

export interface WorkflowActionProposal {
  kind: string;
  action_id: string;
  target_object_id: string | null;
  parameters: Record<string, unknown>;
  justification: string | null;
  summary: string;
  reasoning: unknown;
  preview: unknown;
  what_if_branch: Record<string, unknown> | null;
  auto_apply_on_approval: boolean;
  execution_identity: string;
}

export interface WorkflowApprovalReview {
  decision: string;
  decided_by: string;
  comment: string | null;
  payload: Record<string, unknown>;
  decided_at: string;
}

export interface WorkflowProposalExecution {
  status: string;
  response: unknown;
  error: string | null;
  updated_at: string;
}

export interface WorkflowApprovalPayload {
  request_context?: Record<string, unknown>;
  proposal?: WorkflowActionProposal;
  decision_review?: WorkflowApprovalReview;
  proposal_execution?: WorkflowProposalExecution;
  [key: string]: unknown;
}

export interface WorkflowApproval {
  id: string;
  workflow_id: string;
  workflow_run_id: string;
  step_id: string;
  title: string;
  instructions: string;
  assigned_to: string | null;
  status: string;
  decision: string | null;
  payload: WorkflowApprovalPayload;
  requested_at: string;
  decided_at: string | null;
  decided_by: string | null;
}

export interface WorkflowListResponse {
  data: WorkflowDefinition[];
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

export interface WorkflowRunListResponse {
  data: WorkflowRun[];
  page: number;
  per_page: number;
  total: number;
}

export interface WorkflowApprovalListResponse {
  data: WorkflowApproval[];
  page: number;
  per_page: number;
  total: number;
}

export interface CreateWorkflowParams {
  name: string;
  description?: string;
  status?: string;
  trigger_type: string;
  trigger_config?: Record<string, unknown>;
  steps: WorkflowStep[];
}

export interface UpdateWorkflowParams {
  name?: string;
  description?: string;
  status?: string;
  trigger_type?: string;
  trigger_config?: Record<string, unknown>;
  steps?: WorkflowStep[];
}

export interface ApprovalDecisionResponse {
  approval: WorkflowApproval;
  run: WorkflowRun;
}

export function listWorkflows(params?: { page?: number; per_page?: number; search?: string; trigger_type?: string; status?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  if (params?.search) query.set('search', params.search);
  if (params?.trigger_type) query.set('trigger_type', params.trigger_type);
  if (params?.status) query.set('status', params.status);
  const qs = query.toString();
  return api.get<WorkflowListResponse>(`/workflows${qs ? `?${qs}` : ''}`);
}

export function getWorkflow(id: string) {
  return api.get<WorkflowDefinition>(`/workflows/${id}`);
}

export function createWorkflow(body: CreateWorkflowParams) {
  return api.post<WorkflowDefinition>('/workflows', body);
}

export function updateWorkflow(id: string, body: UpdateWorkflowParams) {
  return api.patch<WorkflowDefinition>(`/workflows/${id}`, body);
}

export function deleteWorkflow(id: string) {
  return api.delete(`/workflows/${id}`);
}

export function listWorkflowRuns(id: string, params?: { page?: number; per_page?: number }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  const qs = query.toString();
  return api.get<WorkflowRunListResponse>(`/workflows/${id}/runs${qs ? `?${qs}` : ''}`);
}

export function startWorkflowRun(id: string, context?: Record<string, unknown>) {
  return api.post<WorkflowRun>(`/workflows/${id}/runs/manual`, { context: context ?? {} });
}

export function triggerWorkflowEvent(eventName: string, context?: Record<string, unknown>) {
  return api.post<{ data: WorkflowRun[]; event_name: string }>(`/workflows/events/${eventName}`, { context: context ?? {} });
}

export function evaluateCronWorkflows() {
  return api.post<{ triggered_runs: number }>('/workflows/triggers/cron/run-due', {});
}

export function listWorkflowApprovals(params?: { page?: number; per_page?: number; status?: string; assigned_to?: string; workflow_id?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  if (params?.status) query.set('status', params.status);
  if (params?.assigned_to) query.set('assigned_to', params.assigned_to);
  if (params?.workflow_id) query.set('workflow_id', params.workflow_id);
  const qs = query.toString();
  return api.get<WorkflowApprovalListResponse>(`/workflows/approvals${qs ? `?${qs}` : ''}`);
}

export function decideWorkflowApproval(id: string, body: { decision: string; comment?: string; payload?: Record<string, unknown> }) {
  return api.post<ApprovalDecisionResponse>(`/workflows/approvals/${id}/decision`, body);
}
