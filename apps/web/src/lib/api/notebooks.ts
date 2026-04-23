import api from './client';

export interface Notebook {
  id: string;
  name: string;
  description: string;
  owner_id: string;
  default_kernel: string;
  created_at: string;
  updated_at: string;
}

export interface Cell {
  id: string;
  notebook_id: string;
  cell_type: string;
  kernel: string;
  source: string;
  position: number;
  last_output: CellOutput | null;
  execution_count: number | null;
  created_at: string;
  updated_at: string;
}

export interface CellOutput {
  output_type: string;
  content: unknown;
  execution_count: number;
}

export interface Session {
  id: string;
  notebook_id: string;
  kernel: string;
  status: string;
  started_by: string;
  created_at: string;
  last_activity: string;
}

// ── Notebooks ──

export function createNotebook(data: { name: string; description?: string; default_kernel?: string }) {
  return api.post<Notebook>('/notebooks', data);
}

export function listNotebooks(params?: { page?: number; per_page?: number; search?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  if (params?.search) query.set('search', params.search);
  const suffix = query.toString();

  return api.get<{ data: Notebook[]; total: number; page: number; per_page: number }>(
    `/notebooks${suffix ? `?${suffix}` : ''}`,
  );
}

export function getNotebook(id: string) {
  return api.get<{ notebook: Notebook; cells: Cell[] }>(`/notebooks/${id}`);
}

export function updateNotebook(id: string, data: { name?: string; description?: string; default_kernel?: string }) {
  return api.put<Notebook>(`/notebooks/${id}`, data);
}

export function deleteNotebook(id: string) {
  return api.delete(`/notebooks/${id}`);
}

// ── Cells ──

export function addCell(notebookId: string, data: { cell_type?: string; kernel?: string; source?: string; position?: number }) {
  return api.post<Cell>(`/notebooks/${notebookId}/cells`, data);
}

export function updateCell(notebookId: string, cellId: string, data: { source?: string; cell_type?: string; kernel?: string; position?: number }) {
  return api.put<Cell>(`/notebooks/${notebookId}/cells/${cellId}`, data);
}

export function deleteCell(notebookId: string, cellId: string) {
  return api.delete(`/notebooks/${notebookId}/cells/${cellId}`);
}

// ── Execution ──

export function executeCell(notebookId: string, cellId: string, sessionId?: string) {
  return api.post<CellOutput>(`/notebooks/${notebookId}/cells/${cellId}/execute`, {
    session_id: sessionId ?? null,
  });
}

export function executeAllCells(notebookId: string, sessionId?: string) {
  return api.post<{ results: { cell_id: string; output: CellOutput }[] }>(`/notebooks/${notebookId}/execute`, {
    session_id: sessionId ?? null,
  });
}

// ── Sessions ──

export function createSession(notebookId: string, kernel?: string) {
  return api.post<Session>(`/notebooks/${notebookId}/sessions`, { kernel });
}

export function listSessions(notebookId: string) {
  return api.get<{ data: Session[] }>(`/notebooks/${notebookId}/sessions`);
}

export function stopSession(notebookId: string, sessionId: string) {
  return api.delete<Session>(`/notebooks/${notebookId}/sessions/${sessionId}`);
}