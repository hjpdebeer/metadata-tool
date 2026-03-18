import type { AxiosResponse } from 'axios';
import api from './api';

// --- Shared types ---

export interface PaginatedResponse<T> {
  data: T[];
  total_count: number;
  page: number;
  page_size: number;
}

// --- Glossary types ---

export interface GlossaryTermListItem {
  term_id: string;
  term_name: string;
  definition: string;
  business_context: string | null;
  abbreviation: string | null;
  domain_id: string | null;
  domain_name: string | null;
  category_id: string | null;
  category_name: string | null;
  status_id: string;
  status_code: string;
  owner_user_id: string | null;
  owner_name: string | null;
  version_number: number;
  created_at: string;
  updated_at: string;
}

export interface GlossaryTerm {
  term_id: string;
  term_name: string;
  definition: string;
  business_context: string | null;
  examples: string | null;
  abbreviation: string | null;
  domain_id: string | null;
  domain_name: string | null;
  category_id: string | null;
  category_name: string | null;
  status_id: string;
  status_code: string;
  owner_user_id: string | null;
  owner_name: string | null;
  steward_user_id: string | null;
  steward_name: string | null;
  version_number: number;
  is_current_version: boolean;
  source_reference: string | null;
  regulatory_reference: string | null;
  created_by: string;
  created_by_name: string | null;
  updated_by: string | null;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
  workflow_instance_id: string | null;
}

export interface CreateGlossaryTermRequest {
  term_name: string;
  definition: string;
  business_context?: string;
  examples?: string;
  abbreviation?: string;
  domain_id?: string;
  category_id?: string;
  source_reference?: string;
  regulatory_reference?: string;
}

export interface UpdateGlossaryTermRequest {
  term_name?: string;
  definition?: string;
  business_context?: string;
  examples?: string;
  abbreviation?: string;
  domain_id?: string;
  category_id?: string;
  source_reference?: string;
  regulatory_reference?: string;
}

export interface GlossaryDomain {
  domain_id: string;
  domain_name: string;
  description: string | null;
  parent_domain_id: string | null;
}

export interface GlossaryCategory {
  category_id: string;
  category_name: string;
  description: string | null;
}

export interface ListTermsParams {
  query?: string;
  domain_id?: string;
  category_id?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

// --- Workflow types ---

export interface WorkflowTask {
  task_id: string;
  instance_id: string;
  task_type: string;
  task_name: string;
  description: string | null;
  assigned_to_user_id: string | null;
  assigned_to_role_id: string | null;
  status: string;
  due_date: string | null;
  completed_at: string | null;
  completed_by: string | null;
  decision: string | null;
  comments: string | null;
}

export interface PendingTask {
  task: WorkflowTask;
  entity_type: string;
  entity_name: string;
  entity_id: string;
  workflow_name: string;
  submitted_by: string;
  submitted_at: string;
}

export interface WorkflowHistoryEntry {
  history_id: string;
  instance_id: string;
  from_state_id: string;
  to_state_id: string;
  from_state_name?: string;
  to_state_name?: string;
  action: string;
  performed_by: string;
  performed_by_name?: string;
  performed_at: string;
  comments: string | null;
}

export interface WorkflowInstanceView {
  instance_id: string;
  workflow_def_id: string;
  entity_type_id: string;
  entity_id: string;
  current_state_id: string;
  current_state_name: string;
  entity_type_name: string;
  initiated_by: string;
  initiated_by_name: string;
  initiated_at: string;
  completed_at: string | null;
  completion_notes: string | null;
  tasks: WorkflowTask[];
  history: WorkflowHistoryEntry[];
}

// --- Stats types ---

export interface Stats {
  glossary_terms: number;
  data_elements: number;
  critical_data_elements: number;
  quality_rules: number;
  applications: number;
  pending_tasks: number;
}

export interface RecentTerm {
  term_id: string;
  term_name: string;
  status_code: string;
  updated_at: string;
}

// --- API functions ---

export const glossaryApi = {
  listTerms(params: ListTermsParams): Promise<AxiosResponse<GlossaryTermListItem[]>> {
    // The backend currently returns Vec<GlossaryTerm>, not paginated.
    // We pass pagination params for when the backend supports them.
    return api.get('/glossary/terms', { params });
  },

  getTerm(id: string): Promise<AxiosResponse<GlossaryTerm>> {
    return api.get(`/glossary/terms/${id}`);
  },

  createTerm(data: CreateGlossaryTermRequest): Promise<AxiosResponse<GlossaryTerm>> {
    return api.post('/glossary/terms', data);
  },

  updateTerm(id: string, data: UpdateGlossaryTermRequest): Promise<AxiosResponse<GlossaryTerm>> {
    return api.put(`/glossary/terms/${id}`, data);
  },

  listDomains(): Promise<AxiosResponse<GlossaryDomain[]>> {
    return api.get('/glossary/domains');
  },

  listCategories(): Promise<AxiosResponse<GlossaryCategory[]>> {
    // Backend categories endpoint may not exist yet; handle gracefully
    return api.get('/glossary/categories');
  },
};

export const workflowApi = {
  getPendingTasks(): Promise<AxiosResponse<PendingTask[]>> {
    return api.get('/workflow/tasks/pending');
  },

  getInstance(instanceId: string): Promise<AxiosResponse<WorkflowInstanceView>> {
    return api.get(`/workflow/instances/${instanceId}`);
  },

  transitionWorkflow(
    instanceId: string,
    action: string,
    comments?: string,
  ): Promise<AxiosResponse<unknown>> {
    return api.post(`/workflow/instances/${instanceId}/transition`, { action, comments });
  },

  completeTask(
    taskId: string,
    decision: string,
    comments?: string,
  ): Promise<AxiosResponse<unknown>> {
    return api.post(`/workflow/tasks/${taskId}/complete`, { decision, comments });
  },
};

export const statsApi = {
  getStats(): Promise<AxiosResponse<Stats>> {
    // The backend may not have a /stats endpoint yet.
    // We'll call individual count endpoints or a combined one.
    return api.get('/stats');
  },
};
