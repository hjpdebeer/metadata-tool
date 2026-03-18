import type { AxiosResponse } from 'axios';
import api from './api';

// --- Types ---

export interface InAppNotification {
  notification_id: string;
  user_id: string;
  title: string;
  message: string;
  link_url: string | null;
  is_read: boolean;
  read_at: string | null;
  entity_type: string | null;
  entity_id: string | null;
  created_at: string;
}

export interface PaginatedNotifications {
  data: InAppNotification[];
  total_count: number;
  page: number;
  page_size: number;
}

export interface UnreadCountResponse {
  count: number;
}

export interface NotificationPreference {
  preference_id: string;
  user_id: string;
  event_type: string;
  email_enabled: boolean;
  in_app_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface UpdatePreferenceItem {
  event_type: string;
  email_enabled: boolean;
  in_app_enabled: boolean;
}

export interface ListNotificationsParams {
  page?: number;
  page_size?: number;
}

// --- API functions ---

export const notificationsApi = {
  listNotifications(
    params?: ListNotificationsParams,
  ): Promise<AxiosResponse<PaginatedNotifications>> {
    return api.get('/notifications', { params });
  },

  markRead(notificationId: string): Promise<AxiosResponse<void>> {
    return api.post(`/notifications/${notificationId}/read`);
  },

  markAllRead(): Promise<AxiosResponse<void>> {
    return api.post('/notifications/read-all');
  },

  getUnreadCount(): Promise<AxiosResponse<UnreadCountResponse>> {
    return api.get('/notifications/unread-count');
  },

  getPreferences(): Promise<AxiosResponse<NotificationPreference[]>> {
    return api.get('/notifications/preferences');
  },

  updatePreferences(
    preferences: UpdatePreferenceItem[],
  ): Promise<AxiosResponse<NotificationPreference[]>> {
    return api.put('/notifications/preferences', { preferences });
  },
};
