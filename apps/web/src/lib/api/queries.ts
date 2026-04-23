import api from './client';

export interface QueryResult {
  columns: { name: string; data_type: string }[];
  rows: string[][];
  total_rows: number;
  execution_time_ms: number;
}

export interface ExplainResult {
  logical_plan: string;
  physical_plan: string;
}

export interface SavedQuery {
  id: string;
  name: string;
  description: string;
  sql: string;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export function executeQuery(sql: string, limit?: number) {
  return api.post<QueryResult>('/queries/execute', { sql, limit });
}

export function explainQuery(sql: string) {
  return api.post<ExplainResult>('/queries/explain', { sql });
}

export function createSavedQuery(params: { name: string; description?: string; sql: string }) {
  return api.post<SavedQuery>('/queries/saved', params);
}

export function listSavedQueries(params?: { page?: number; search?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.search) query.set('search', params.search);
  const qs = query.toString();
  return api.get<{ data: SavedQuery[] }>(`/queries/saved${qs ? `?${qs}` : ''}`);
}

export function deleteSavedQuery(id: string) {
  return api.delete(`/queries/saved/${id}`);
}
