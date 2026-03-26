import type { AxiosResponse } from 'axios';
import api from './api';

// ---------------------------------------------------------------------------
// Types: System Settings
// ---------------------------------------------------------------------------

export interface SystemSetting {
  key: string;
  value: string;
  is_encrypted: boolean;
  category: string;
  display_name: string;
  description: string | null;
  validation_regex: string | null;
  is_set: boolean;
  updated_at: string;
  updated_by_name: string | null;
}

export interface SettingsListResponse {
  settings: SystemSetting[];
}

export interface UpdateSettingResponse {
  key: string;
  is_set: boolean;
  updated_at: string;
}

export interface RevealSettingResponse {
  key: string;
  value: string;
}

export interface TestConnectionResponse {
  success: boolean;
  message: string;
}

// ---------------------------------------------------------------------------
// Types: Lookup Tables
// ---------------------------------------------------------------------------

export interface LookupRow {
  id: string;
  code: string | null;
  name: string;
  description: string | null;
  extra?: Record<string, unknown>;
}

export interface LookupListResponse {
  data: LookupRow[];
  total_count: number;
}

export interface LookupRowRequest {
  code?: string;
  name: string;
  description?: string;
  extra?: Record<string, unknown>;
}

export interface UsageCountResponse {
  usage_count: number;
  table_name: string;
}

export interface LookupListParams {
  search?: string;
  page?: number;
  page_size?: number;
}

// ---------------------------------------------------------------------------
// Lookup table metadata
// ---------------------------------------------------------------------------

export interface LookupTableMeta {
  key: string;
  label: string;
  hasCode: boolean;
}

export const LOOKUP_TABLES: LookupTableMeta[] = [
  // Glossary
  { key: 'domains', label: 'Domains', hasCode: true },
  { key: 'categories', label: 'Categories', hasCode: false },
  { key: 'term-types', label: 'Term Types', hasCode: true },
  { key: 'tags', label: 'Tags', hasCode: false },
  { key: 'regulatory-tags', label: 'Regulatory Tags', hasCode: true },
  { key: 'subject-areas', label: 'Subject Areas', hasCode: true },
  // Data Dictionary
  { key: 'classifications', label: 'Data Classifications', hasCode: true },
  // Applications
  { key: 'app-classifications', label: 'App Classifications', hasCode: true },
  { key: 'dr-tiers', label: 'DR Tiers', hasCode: true },
  { key: 'lifecycle-stages', label: 'Lifecycle Stages', hasCode: true },
  { key: 'criticality-tiers', label: 'Criticality Tiers', hasCode: true },
  { key: 'risk-ratings', label: 'Risk Ratings', hasCode: true },
  // Business Processes
  { key: 'process-categories', label: 'Process Categories', hasCode: false },
  // Shared
  { key: 'organisational-units', label: 'Organisational Units', hasCode: true },
  { key: 'review-frequencies', label: 'Review Frequencies', hasCode: true },
  { key: 'confidence-levels', label: 'Confidence Levels', hasCode: true },
  { key: 'visibility-levels', label: 'Visibility Levels', hasCode: true },
  { key: 'units-of-measure', label: 'Units of Measure', hasCode: true },
  { key: 'languages', label: 'Languages', hasCode: true },
];

// ---------------------------------------------------------------------------
// API functions: Settings
// ---------------------------------------------------------------------------

export const adminApi = {
  // Settings
  listSettings(): Promise<AxiosResponse<SettingsListResponse>> {
    return api.get('/admin/settings');
  },

  updateSetting(key: string, value: string): Promise<AxiosResponse<UpdateSettingResponse>> {
    return api.put(`/admin/settings/${key}`, { value });
  },

  revealSetting(key: string): Promise<AxiosResponse<RevealSettingResponse>> {
    return api.get(`/admin/settings/${key}/reveal`);
  },

  testConnection(key: string): Promise<AxiosResponse<TestConnectionResponse>> {
    return api.post(`/admin/settings/test-connection/${key}`);
  },

  // Lookup tables
  listLookup(
    tableName: string,
    params?: LookupListParams,
  ): Promise<AxiosResponse<LookupListResponse>> {
    return api.get(`/admin/lookups/${tableName}`, { params });
  },

  createLookup(tableName: string, data: LookupRowRequest): Promise<AxiosResponse<LookupRow>> {
    return api.post(`/admin/lookups/${tableName}`, data);
  },

  updateLookup(
    tableName: string,
    id: string,
    data: LookupRowRequest,
  ): Promise<AxiosResponse<LookupRow>> {
    return api.put(`/admin/lookups/${tableName}/${id}`, data);
  },

  deleteLookup(tableName: string, id: string): Promise<AxiosResponse<void>> {
    return api.delete(`/admin/lookups/${tableName}/${id}`);
  },

  getUsageCount(tableName: string, id: string): Promise<AxiosResponse<UsageCountResponse>> {
    return api.get(`/admin/lookups/${tableName}/${id}/usage-count`);
  },
};
