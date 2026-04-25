import api from './client';

export interface Dataset {
  id: string;
  name: string;
  description: string;
  format: string;
  storage_path: string;
  size_bytes: number;
  row_count: number;
  owner_id: string;
  tags: string[];
  current_version: number;
  active_branch: string;
  created_at: string;
  updated_at: string;
}

export interface DatasetListResponse {
  data: Dataset[];
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

export interface CreateDatasetParams {
  name: string;
  description?: string;
  format?: string;
  tags?: string[];
}

export interface UpdateDatasetParams {
  name?: string;
  description?: string;
  owner_id?: string;
  tags?: string[];
}

export interface DatasetVersion {
  id: string;
  dataset_id: string;
  version: number;
  message: string;
  size_bytes: number;
  row_count: number;
  storage_path: string;
  transaction_id?: string | null;
  created_at: string;
}

export interface DatasetPreviewResponse {
  dataset_id: string;
  version?: number;
  size_bytes?: number;
  format?: string;
  branch?: string | null;
  storage_path?: string;
  limit?: number;
  offset?: number;
  row_count?: number;
  rows?: Array<Record<string, unknown>>;
  columns?: Array<{
    name: string;
    field_type?: string;
    data_type?: string;
    nullable?: boolean;
  }>;
  total_rows?: number;
  warnings?: string[];
  errors?: string[];
  message?: string;
}

export interface DatasetBranch {
  id: string;
  dataset_id: string;
  name: string;
  version: number;
  base_version?: number;
  description: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface DatasetView {
  id: string;
  dataset_id: string;
  name: string;
  description: string;
  sql_text: string;
  source_branch?: string | null;
  source_version?: number | null;
  materialized: boolean;
  refresh_on_source_update: boolean;
  format: string;
  current_version: number;
  storage_path?: string | null;
  row_count: number;
  schema_fields: unknown;
  last_refreshed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateDatasetViewParams {
  name: string;
  description?: string;
  sql: string;
  source_branch?: string;
  source_version?: number;
  materialized?: boolean;
  refresh_on_source_update?: boolean;
}

export interface DatasetFilesystemEntry {
  entry_type: 'directory' | 'file';
  name: string;
  path: string;
  storage_path?: string;
  size_bytes?: number;
  last_modified?: string;
  content_type?: string | null;
  metadata: Record<string, unknown>;
}

export interface DatasetFilesystemResponse {
  dataset_id: string;
  requested_path: string;
  root: string;
  current_version: number;
  active_branch: string;
  entries: DatasetFilesystemEntry[];
  items: DatasetFilesystemEntry[];
  breadcrumbs: Array<{ name: string; path: string }>;
  sections: {
    versions: number;
    branches: number;
    views: number;
  };
}

export interface DatasetTransaction {
  id: string;
  dataset_id: string;
  view_id?: string | null;
  operation: string;
  branch_name?: string | null;
  status: string;
  summary: string;
  metadata: Record<string, unknown>;
  created_at: string;
  committed_at?: string | null;
}

export interface CreateDatasetBranchParams {
  name: string;
  source_version?: number;
  description?: string;
}

export interface CatalogTagFacet {
  value: string;
  count: number;
}

export interface CatalogOwnerFacet {
  owner_id: string;
  count: number;
}

export interface DatasetCatalogFacets {
  tags: CatalogTagFacet[];
  owners: CatalogOwnerFacet[];
}

export interface DatasetValueCount {
  value: string;
  count: number;
}

export interface DatasetColumnProfile {
  name: string;
  field_type: string;
  nullable: boolean;
  null_count: number;
  null_rate: number;
  distinct_count: number;
  uniqueness_rate: number;
  sample_values: DatasetValueCount[];
  min_value: string | null;
  max_value: string | null;
  average_value: number | null;
}

export interface DatasetRuleResult {
  rule_id: string;
  name: string;
  rule_type: string;
  severity: string;
  passed: boolean;
  measured_value: string | null;
  message: string;
}

export interface DatasetQualityProfile {
  row_count: number;
  column_count: number;
  duplicate_rows: number;
  completeness_ratio: number;
  uniqueness_ratio: number;
  generated_at: string;
  columns: DatasetColumnProfile[];
  rule_results: DatasetRuleResult[];
}

export interface DatasetQualityRule {
  id: string;
  dataset_id: string;
  name: string;
  rule_type: string;
  severity: string;
  config: Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface DatasetQualityHistoryEntry {
  id: string;
  dataset_id: string;
  score: number;
  passed_rules: number;
  failed_rules: number;
  alerts_count: number;
  created_at: string;
}

export interface DatasetQualityAlert {
  id: string;
  dataset_id: string;
  level: string;
  kind: string;
  message: string;
  status: string;
  details: Record<string, unknown>;
  created_at: string;
  resolved_at: string | null;
}

export interface DatasetQualityResponse {
  profile: DatasetQualityProfile | null;
  score: number | null;
  history: DatasetQualityHistoryEntry[];
  alerts: DatasetQualityAlert[];
  rules: DatasetQualityRule[];
  profiled_at: string | null;
}

export interface CreateDatasetQualityRuleParams {
  name: string;
  rule_type: string;
  severity?: string;
  enabled?: boolean;
  config: Record<string, unknown>;
}

export interface UpdateDatasetQualityRuleParams {
  name?: string;
  severity?: string;
  enabled?: boolean;
  config?: Record<string, unknown>;
}

export function listDatasets(params?: { page?: number; per_page?: number; search?: string; tag?: string; owner_id?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  if (params?.search) query.set('search', params.search);
  if (params?.tag) query.set('tag', params.tag);
  if (params?.owner_id) query.set('owner_id', params.owner_id);
  const qs = query.toString();
  return api.get<DatasetListResponse>(`/datasets${qs ? `?${qs}` : ''}`);
}

export function getCatalogFacets() {
  return api.get<DatasetCatalogFacets>('/datasets/catalog/facets');
}

export function getDataset(id: string) {
  return api.get<Dataset>(`/datasets/${id}`);
}

export function previewDataset(datasetId: string, params?: { limit?: number; offset?: number }) {
  const query = new URLSearchParams();
  if (params?.limit) query.set('limit', String(params.limit));
  if (params?.offset) query.set('offset', String(params.offset));
  const qs = query.toString();
  return api.get<DatasetPreviewResponse>(`/datasets/${datasetId}/preview${qs ? `?${qs}` : ''}`);
}

export function createDataset(params: CreateDatasetParams) {
  return api.post<Dataset>('/datasets', params);
}

export function updateDataset(id: string, params: UpdateDatasetParams) {
  return api.patch<Dataset>(`/datasets/${id}`, params);
}

export function deleteDataset(id: string) {
  return api.delete(`/datasets/${id}`);
}

export function getVersions(datasetId: string) {
  return api.get<DatasetVersion[]>(`/datasets/${datasetId}/versions`);
}

export function listDatasetTransactions(datasetId: string) {
  return api.get<DatasetTransaction[]>(`/datasets/${datasetId}/transactions`);
}

export function listBranches(datasetId: string) {
  return api.get<DatasetBranch[]>(`/datasets/${datasetId}/branches`);
}

export function createDatasetBranch(datasetId: string, params: CreateDatasetBranchParams) {
  return api.post<DatasetBranch>(`/datasets/${datasetId}/branches`, params);
}

export function checkoutDatasetBranch(datasetId: string, branchName: string) {
  return api.post<Dataset>(`/datasets/${datasetId}/branches/${encodeURIComponent(branchName)}/checkout`, {});
}

export function listDatasetViews(datasetId: string) {
  return api.get<DatasetView[]>(`/datasets/${datasetId}/views`);
}

export function getDatasetView(datasetId: string, viewId: string) {
  return api.get<DatasetView>(`/datasets/${datasetId}/views/${viewId}`);
}

export function createDatasetView(datasetId: string, params: CreateDatasetViewParams) {
  return api.post<DatasetView>(`/datasets/${datasetId}/views`, params);
}

export function refreshDatasetView(datasetId: string, viewId: string) {
  return api.post<DatasetView>(`/datasets/${datasetId}/views/${viewId}/refresh`, {});
}

export function previewDatasetView(datasetId: string, viewId: string, params?: { limit?: number; offset?: number }) {
  const query = new URLSearchParams();
  if (params?.limit) query.set('limit', String(params.limit));
  if (params?.offset) query.set('offset', String(params.offset));
  const qs = query.toString();
  return api.get<Record<string, unknown>>(`/datasets/${datasetId}/views/${viewId}/preview${qs ? `?${qs}` : ''}`);
}

export function listDatasetFilesystem(datasetId: string, params?: { path?: string }) {
  const query = new URLSearchParams();
  if (params?.path) query.set('path', params.path);
  const qs = query.toString();
  return api.get<DatasetFilesystemResponse>(`/datasets/${datasetId}/filesystem${qs ? `?${qs}` : ''}`);
}

export function getDatasetQuality(datasetId: string) {
  return api.get<DatasetQualityResponse>(`/datasets/${datasetId}/quality`);
}

export function refreshDatasetQualityProfile(datasetId: string) {
  return api.post<DatasetQualityResponse>(`/datasets/${datasetId}/quality/profile`, {});
}

export function createDatasetQualityRule(datasetId: string, params: CreateDatasetQualityRuleParams) {
  return api.post<DatasetQualityResponse>(`/datasets/${datasetId}/quality/rules`, params);
}

export function updateDatasetQualityRule(datasetId: string, ruleId: string, params: UpdateDatasetQualityRuleParams) {
  return api.patch<DatasetQualityResponse>(`/datasets/${datasetId}/quality/rules/${ruleId}`, params);
}

export function deleteDatasetQualityRule(datasetId: string, ruleId: string) {
  return api.delete<DatasetQualityResponse>(`/datasets/${datasetId}/quality/rules/${ruleId}`);
}

export async function uploadData(datasetId: string, file: File) {
  const formData = new FormData();
  formData.append('file', file);
  const headers: Record<string, string> = {};
  const authHeader = api.authorizationHeaders().Authorization;
  if (authHeader) {
    headers.Authorization = authHeader;
  }
  const response = await fetch(`/api/v1/datasets/${datasetId}/upload`, {
    method: 'POST',
    headers,
    body: formData,
  });
  if (!response.ok) throw new Error('Upload failed');
  return response.json();
}
