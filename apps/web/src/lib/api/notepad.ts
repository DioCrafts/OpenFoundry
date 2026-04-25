import api from './client';

export interface NotepadDocument {
  id: string;
  title: string;
  description: string;
  owner_id: string;
  content: string;
  template_key: string | null;
  widgets: Array<Record<string, unknown>>;
  last_indexed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface NotepadPresence {
  id: string;
  document_id: string;
  user_id: string;
  session_id: string;
  display_name: string;
  cursor_label: string;
  color: string;
  last_seen_at: string;
}

export interface NotepadExportPayload {
  file_name: string;
  mime_type: string;
  title: string;
  html: string;
  preview_excerpt: string;
}

export function listNotepadDocuments(params?: { page?: number; per_page?: number; search?: string }) {
  const query = new URLSearchParams();
  if (params?.page) query.set('page', String(params.page));
  if (params?.per_page) query.set('per_page', String(params.per_page));
  if (params?.search) query.set('search', params.search);
  const qs = query.toString();

  return api.get<{ data: NotepadDocument[]; total: number; page: number; per_page: number }>(
    `/notepad/documents${qs ? `?${qs}` : ''}`,
  );
}

export function createNotepadDocument(body: {
  title: string;
  description?: string;
  content?: string;
  template_key?: string;
  widgets?: Array<Record<string, unknown>>;
}) {
  return api.post<NotepadDocument>('/notepad/documents', body);
}

export function getNotepadDocument(id: string) {
  return api.get<NotepadDocument>(`/notepad/documents/${id}`);
}

export function updateNotepadDocument(id: string, body: Partial<{
  title: string;
  description: string;
  content: string;
  template_key: string;
  widgets: Array<Record<string, unknown>>;
  last_indexed_at: string | null;
}>) {
  return api.patch<NotepadDocument>(`/notepad/documents/${id}`, body);
}

export function deleteNotepadDocument(id: string) {
  return api.delete(`/notepad/documents/${id}`);
}

export function listNotepadPresence(id: string) {
  return api.get<{ data: NotepadPresence[] }>(`/notepad/documents/${id}/presence`);
}

export function upsertNotepadPresence(id: string, body: {
  session_id: string;
  display_name: string;
  cursor_label?: string;
  color?: string;
}) {
  return api.post<NotepadPresence>(`/notepad/documents/${id}/presence`, body);
}

export function exportNotepadDocument(id: string) {
  return api.get<NotepadExportPayload>(`/notepad/documents/${id}/export`);
}
