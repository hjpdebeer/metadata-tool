import type { AxiosResponse } from 'axios';
import api from './api';

// --- Application Classification types ---

export interface ApplicationClassification {
  classification_id: string;
  classification_code: string;
  classification_name: string;
  description: string | null;
  display_order: number;
}

// --- Application types ---

export interface Application {
  application_id: string;
  application_name: string;
  application_code: string;
  description: string;
  classification_id: string | null;
  status_id: string;
  business_owner_id: string | null;
  technical_owner_id: string | null;
  vendor: string | null;
  version: string | null;
  deployment_type: string | null;
  technology_stack: unknown | null;
  is_critical: boolean;
  criticality_rationale: string | null;
  go_live_date: string | null;
  retirement_date: string | null;
  documentation_url: string | null;
  created_by: string;
  updated_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface ApplicationListItem {
  application_id: string;
  application_name: string;
  application_code: string;
  description: string;
  classification_name: string | null;
  classification_code: string | null;
  vendor: string | null;
  deployment_type: string | null;
  is_critical: boolean;
  status_code: string;
  status_name: string | null;
  business_owner_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface ApplicationFullView {
  application_id: string;
  application_name: string;
  application_code: string;
  description: string;
  classification_id: string | null;
  classification_name: string | null;
  status_id: string;
  status_code: string;
  business_owner_id: string | null;
  business_owner_name: string | null;
  technical_owner_id: string | null;
  technical_owner_name: string | null;
  vendor: string | null;
  version: string | null;
  deployment_type: string | null;
  technology_stack: unknown | null;
  is_critical: boolean;
  criticality_rationale: string | null;
  go_live_date: string | null;
  retirement_date: string | null;
  documentation_url: string | null;
  created_by: string;
  created_by_name: string | null;
  updated_by: string | null;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
  workflow_instance_id: string | null;
  data_elements_count: number;
  interfaces_count: number;
  linked_processes: string[];
}

export interface ApplicationInterface {
  interface_id: string;
  source_app_id: string;
  target_app_id: string;
  source_app_name: string;
  target_app_name: string;
  interface_name: string;
  interface_type: string;
  protocol: string | null;
  frequency: string | null;
  description: string | null;
}

export interface ApplicationDataElementLink {
  id: string;
  application_id: string;
  element_id: string;
  element_name: string;
  element_code: string;
  usage_type: string;
  is_authoritative_source: boolean;
  is_cde: boolean;
  description: string | null;
  created_at: string;
}

// --- Request types ---

export interface CreateApplicationRequest {
  application_name: string;
  application_code: string;
  description: string;
  classification_id?: string;
  vendor?: string;
  version?: string;
  deployment_type?: string;
  technology_stack?: string;
  is_critical?: boolean;
  criticality_rationale?: string;
  go_live_date?: string;
  documentation_url?: string;
}

export interface UpdateApplicationRequest {
  application_name?: string;
  description?: string;
  classification_id?: string;
  vendor?: string;
  version?: string;
  deployment_type?: string;
  technology_stack?: string;
  is_critical?: boolean;
  criticality_rationale?: string;
  retirement_date?: string;
  documentation_url?: string;
}

export interface LinkDataElementRequest {
  element_id: string;
  usage_type: string;
  is_authoritative_source?: boolean;
  description?: string;
}

export interface ListApplicationsParams {
  query?: string;
  classification_id?: string;
  status?: string;
  deployment_type?: string;
  is_critical?: boolean;
  page?: number;
  page_size?: number;
}

// --- API functions ---

export const applicationsApi = {
  listApplications(params: ListApplicationsParams): Promise<AxiosResponse<ApplicationListItem[]>> {
    return api.get('/applications', { params });
  },

  getApplication(id: string): Promise<AxiosResponse<ApplicationFullView>> {
    return api.get(`/applications/${id}`);
  },

  createApplication(data: CreateApplicationRequest): Promise<AxiosResponse<Application>> {
    return api.post('/applications', data);
  },

  updateApplication(id: string, data: UpdateApplicationRequest): Promise<AxiosResponse<Application>> {
    return api.put(`/applications/${id}`, data);
  },

  listClassifications(): Promise<AxiosResponse<ApplicationClassification[]>> {
    return api.get('/applications/classifications');
  },

  linkDataElement(appId: string, data: LinkDataElementRequest): Promise<AxiosResponse<ApplicationDataElementLink>> {
    return api.post(`/applications/${appId}/data-elements`, data);
  },

  listAppElements(appId: string): Promise<AxiosResponse<ApplicationDataElementLink[]>> {
    return api.get(`/applications/${appId}/data-elements`);
  },

  listInterfaces(appId: string): Promise<AxiosResponse<ApplicationInterface[]>> {
    return api.get(`/applications/${appId}/interfaces`);
  },
};
