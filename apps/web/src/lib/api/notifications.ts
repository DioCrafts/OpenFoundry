import api from './client';

export interface UserNotification {
  id: string;
  user_id: string | null;
  title: string;
  body: string;
  category: string;
  severity: string;
  status: string;
  channels: string[];
  metadata: Record<string, unknown>;
  created_at: string;
  read_at: string | null;
}

export interface NotificationListResponse {
  data: UserNotification[];
  unread_count: number;
}

export interface NotificationPreference {
  user_id: string;
  in_app_enabled: boolean;
  email_enabled: boolean;
  email_address: string | null;
  slack_webhook_url: string | null;
  teams_webhook_url: string | null;
  digest_frequency: string;
  quiet_hours: Record<string, unknown>;
  updated_at: string;
}

export interface NotificationSocketEvent {
  kind: string;
  user_id?: string | null;
  notification?: UserNotification | null;
  unread_count: number;
  data?: UserNotification[];
}

export interface NotificationSocketTicket {
  ticket: string;
  expires_in: number;
}

export function listNotifications(params?: { status?: string; limit?: number }) {
  const query = new URLSearchParams();
  if (params?.status) query.set('status', params.status);
  if (params?.limit) query.set('limit', String(params.limit));
  const qs = query.toString();
  return api.get<NotificationListResponse>(`/notifications${qs ? `?${qs}` : ''}`);
}

export function markNotificationRead(id: string) {
  return api.patch<{ notification: UserNotification; unread_count: number }>(`/notifications/${id}/read`, {});
}

export function markAllNotificationsRead() {
  return api.post<{ unread_count: number }>('/notifications/read-all', {});
}

export function getNotificationPreferences() {
  return api.get<NotificationPreference>('/notifications/preferences');
}

export function updateNotificationPreferences(body: Partial<Omit<NotificationPreference, 'user_id' | 'updated_at'>>) {
  return api.put<NotificationPreference>('/notifications/preferences', body);
}

export function issueNotificationSocketTicket() {
  return api.post<NotificationSocketTicket>('/notifications/ws-ticket', {});
}

export function sendNotification(body: {
  user_id?: string;
  title: string;
  body: string;
  severity?: string;
  category?: string;
  channels?: string[];
  metadata?: Record<string, unknown>;
}) {
  return api.post<UserNotification>('/notifications/send', body);
}
