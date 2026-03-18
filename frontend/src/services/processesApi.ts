import type { AxiosResponse } from 'axios';
import api from './api';

// --- Process Category types ---

export interface ProcessCategory {
  category_id: string;
  category_name: string;
  description: string | null;
  parent_category_id: string | null;
}

// --- Business Process types ---

export interface BusinessProcess {
  process_id: string;
  process_name: string;
  process_code: string;
  description: string;
  detailed_description: string | null;
  category_id: string | null;
  status_id: string;
  owner_user_id: string | null;
  parent_process_id: string | null;
  is_critical: boolean;
  criticality_rationale: string | null;
  frequency: string | null;
  regulatory_requirement: string | null;
  sla_description: string | null;
  documentation_url: string | null;
  created_by: string;
  updated_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface BusinessProcessListItem {
  process_id: string;
  process_name: string;
  process_code: string;
  description: string;
  category_name: string | null;
  is_critical: boolean;
  status_code: string;
  status_name: string | null;
  owner_name: string | null;
  frequency: string | null;
  created_at: string;
  updated_at: string;
}

export interface BusinessProcessFullView {
  process_id: string;
  process_name: string;
  process_code: string;
  description: string;
  detailed_description: string | null;
  category_id: string | null;
  category_name: string | null;
  status_id: string;
  status_code: string;
  owner_user_id: string | null;
  owner_name: string | null;
  parent_process_id: string | null;
  parent_process_name: string | null;
  is_critical: boolean;
  criticality_rationale: string | null;
  frequency: string | null;
  regulatory_requirement: string | null;
  sla_description: string | null;
  documentation_url: string | null;
  created_by: string;
  created_by_name: string | null;
  updated_by: string | null;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
  workflow_instance_id: string | null;
  steps: ProcessStep[];
  data_elements_count: number;
  linked_applications: string[];
  sub_processes: BusinessProcess[];
}

export interface ProcessStep {
  step_id: string;
  process_id: string;
  step_number: number;
  step_name: string;
  description: string | null;
  responsible_role: string | null;
  application_id: string | null;
  application_name: string | null;
  input_data_elements: unknown | null;
  output_data_elements: unknown | null;
  created_at: string;
  updated_at: string;
}

export interface ProcessDataElementLink {
  id: string;
  process_id: string;
  element_id: string;
  element_name: string;
  element_code: string;
  usage_type: string;
  is_required: boolean;
  is_cde: boolean;
  description: string | null;
  created_at: string;
}

export interface ProcessApplicationLink {
  id: string;
  process_id: string;
  application_id: string;
  application_name: string;
  application_code: string;
  role_in_process: string | null;
  description: string | null;
  created_at: string;
}

// --- Request types ---

export interface CreateProcessRequest {
  process_name: string;
  process_code: string;
  description: string;
  detailed_description?: string;
  category_id?: string;
  parent_process_id?: string;
  is_critical?: boolean;
  criticality_rationale?: string;
  frequency?: string;
  regulatory_requirement?: string;
  sla_description?: string;
  documentation_url?: string;
}

export interface UpdateProcessRequest {
  process_name?: string;
  process_code?: string;
  description?: string;
  detailed_description?: string;
  category_id?: string;
  parent_process_id?: string;
  is_critical?: boolean;
  criticality_rationale?: string;
  frequency?: string;
  regulatory_requirement?: string;
  sla_description?: string;
  documentation_url?: string;
}

export interface CreateStepRequest {
  step_number: number;
  step_name: string;
  description?: string;
  responsible_role?: string;
  application_id?: string;
}

export interface LinkElementRequest {
  element_id: string;
  usage_type: string;
  is_required?: boolean;
  description?: string;
}

export interface LinkApplicationRequest {
  application_id: string;
  role_in_process?: string;
  description?: string;
}

export interface ListProcessesParams {
  query?: string;
  category_id?: string;
  status?: string;
  is_critical?: boolean;
  page?: number;
  page_size?: number;
}

// --- API functions ---

export const processesApi = {
  listProcesses(params: ListProcessesParams): Promise<AxiosResponse<BusinessProcessListItem[]>> {
    return api.get('/processes', { params });
  },

  getProcess(id: string): Promise<AxiosResponse<BusinessProcessFullView>> {
    return api.get(`/processes/${id}`);
  },

  createProcess(data: CreateProcessRequest): Promise<AxiosResponse<BusinessProcess>> {
    return api.post('/processes', data);
  },

  updateProcess(id: string, data: UpdateProcessRequest): Promise<AxiosResponse<BusinessProcess>> {
    return api.put(`/processes/${id}`, data);
  },

  listCriticalProcesses(): Promise<AxiosResponse<BusinessProcessListItem[]>> {
    return api.get('/processes/critical');
  },

  listCategories(): Promise<AxiosResponse<ProcessCategory[]>> {
    return api.get('/processes/categories');
  },

  addStep(processId: string, data: CreateStepRequest): Promise<AxiosResponse<ProcessStep>> {
    return api.post(`/processes/${processId}/steps`, data);
  },

  listSteps(processId: string): Promise<AxiosResponse<ProcessStep[]>> {
    return api.get(`/processes/${processId}/steps`);
  },

  linkDataElement(processId: string, data: LinkElementRequest): Promise<AxiosResponse<ProcessDataElementLink>> {
    return api.post(`/processes/${processId}/data-elements`, data);
  },

  listProcessElements(processId: string): Promise<AxiosResponse<ProcessDataElementLink[]>> {
    return api.get(`/processes/${processId}/data-elements`);
  },

  linkApplication(processId: string, data: LinkApplicationRequest): Promise<AxiosResponse<ProcessApplicationLink>> {
    return api.post(`/processes/${processId}/applications`, data);
  },

  listProcessApplications(processId: string): Promise<AxiosResponse<ProcessApplicationLink[]>> {
    return api.get(`/processes/${processId}/applications`);
  },
};
