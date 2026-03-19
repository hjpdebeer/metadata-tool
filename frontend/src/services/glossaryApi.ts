import type { AxiosResponse } from 'axios';
import api from './api';

// --- Shared types ---

export interface PaginatedResponse<T> {
  data: T[];
  total_count: number;
  page: number;
  page_size: number;
}

// --- Lookup types for the enhanced 45-field glossary ---

export interface GlossaryTermType {
  term_type_id: string;
  type_code: string;
  type_name: string;
  description: string | null;
}

export interface GlossaryReviewFrequency {
  frequency_id: string;
  frequency_code: string;
  frequency_name: string;
  months_interval: number;
}

export interface GlossaryConfidenceLevel {
  confidence_id: string;
  level_code: string;
  level_name: string;
  description: string | null;
}

export interface GlossaryVisibilityLevel {
  visibility_id: string;
  visibility_code: string;
  visibility_name: string;
  description: string | null;
}

export interface GlossaryUnitOfMeasure {
  unit_id: string;
  unit_code: string;
  unit_name: string;
  unit_symbol: string | null;
}

export interface GlossaryRegulatoryTag {
  tag_id: string;
  tag_code: string;
  tag_name: string;
  description: string | null;
}

export interface GlossarySubjectArea {
  area_id: string;
  area_code: string;
  area_name: string;
  description: string | null;
}

export interface GlossaryLanguage {
  language_id: string;
  language_code: string;
  language_name: string;
}

export interface GlossaryTag {
  tag_id: string;
  tag_name: string;
}

export interface DataClassificationRef {
  classification_id: string;
  classification_code: string;
  classification_name: string;
  description: string | null;
}

// --- Glossary term types ---

export interface GlossaryTermListItem {
  term_id: string;
  term_name: string;
  term_code: string | null;
  definition: string;
  business_context: string | null;
  abbreviation: string | null;
  domain_id: string | null;
  domain_name: string | null;
  category_id: string | null;
  category_name: string | null;
  term_type_id: string | null;
  term_type_name: string | null;
  is_cde: boolean;
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
  term_code: string | null;
  definition: string;
  definition_notes: string | null;
  counter_examples: string | null;
  formula: string | null;
  business_context: string | null;
  examples: string | null;
  abbreviation: string | null;
  domain_id: string | null;
  domain_name: string | null;
  category_id: string | null;
  category_name: string | null;
  term_type_id: string | null;
  unit_of_measure_id: string | null;
  classification_id: string | null;
  status_id: string;
  status_code: string;
  owner_user_id: string | null;
  owner_name: string | null;
  steward_user_id: string | null;
  steward_name: string | null;
  domain_owner_user_id: string | null;
  approver_user_id: string | null;
  organisational_unit: string | null;
  parent_term_id: string | null;
  version_number: number;
  is_current_version: boolean;
  is_cde: boolean;
  golden_source: string | null;
  used_in_reports: string | null;
  used_in_policies: string | null;
  regulatory_reporting_usage: string | null;
  review_frequency_id: string | null;
  confidence_level_id: string | null;
  visibility_id: string | null;
  language_id: string | null;
  source_reference: string | null;
  regulatory_reference: string | null;
  external_reference: string | null;
  approved_at: string | null;
  next_review_date: string | null;
  created_by: string;
  created_by_name: string | null;
  updated_by: string | null;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
  workflow_instance_id: string | null;
}

/** Enhanced detail view with resolved names and junction data */
export interface GlossaryTermDetailView {
  term: GlossaryTerm;
  // Resolved lookup display names
  term_type_name: string | null;
  unit_of_measure_name: string | null;
  unit_of_measure_symbol: string | null;
  classification_name: string | null;
  review_frequency_name: string | null;
  confidence_level_name: string | null;
  visibility_name: string | null;
  language_name: string | null;
  domain_owner_name: string | null;
  approver_name: string | null;
  parent_term_name: string | null;
  // Junction data
  regulatory_tags: { tag_id: string; tag_code: string; tag_name: string }[];
  subject_areas: { area_id: string; area_code: string; area_name: string }[];
  tags: { tag_id: string; tag_name: string }[];
  linked_processes: { process_id: string; process_name: string }[];
  // Relationships (from glossary_term_relationships)
  related_terms: { term_id: string; term_name: string; relationship_type: string; relationship_type_name: string }[];
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
  definition_notes?: string;
  counter_examples?: string;
  formula?: string;
  business_context?: string;
  examples?: string;
  abbreviation?: string;
  domain_id?: string;
  category_id?: string;
  term_type_id?: string;
  unit_of_measure_id?: string;
  classification_id?: string;
  owner_user_id?: string;
  steward_user_id?: string;
  domain_owner_user_id?: string;
  approver_user_id?: string;
  organisational_unit?: string;
  review_frequency_id?: string;
  is_cde?: boolean;
  golden_source?: string;
  confidence_level_id?: string;
  visibility_id?: string;
  language_id?: string;
  used_in_reports?: string;
  used_in_policies?: string;
  regulatory_reporting_usage?: string;
  source_reference?: string;
  regulatory_reference?: string;
  external_reference?: string;
}

export interface GlossaryDomain {
  domain_id: string;
  domain_name: string;
  domain_code: string;
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
  term_type_id?: string;
  status?: string;
  is_cde?: boolean;
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
  // ----- Term CRUD -----

  listTerms(params: ListTermsParams): Promise<AxiosResponse<GlossaryTermListItem[]>> {
    return api.get('/glossary/terms', { params });
  },

  getTerm(id: string): Promise<AxiosResponse<GlossaryTerm>> {
    return api.get(`/glossary/terms/${id}`);
  },

  /** Enhanced detail endpoint returning resolved names and junction data */
  getTermDetail(id: string): Promise<AxiosResponse<GlossaryTermDetailView>> {
    return api.get(`/glossary/terms/${id}/detail`);
  },

  createTerm(data: CreateGlossaryTermRequest): Promise<AxiosResponse<GlossaryTerm>> {
    return api.post('/glossary/terms', data);
  },

  updateTerm(id: string, data: UpdateGlossaryTermRequest): Promise<AxiosResponse<GlossaryTerm>> {
    return api.put(`/glossary/terms/${id}`, data);
  },

  // ----- Reference data -----

  listDomains(): Promise<AxiosResponse<GlossaryDomain[]>> {
    return api.get('/glossary/domains');
  },

  listCategories(): Promise<AxiosResponse<GlossaryCategory[]>> {
    return api.get('/glossary/categories');
  },

  // ----- New lookup endpoints -----

  listTermTypes(): Promise<AxiosResponse<GlossaryTermType[]>> {
    return api.get('/glossary/lookups/term-types');
  },

  listReviewFrequencies(): Promise<AxiosResponse<GlossaryReviewFrequency[]>> {
    return api.get('/glossary/lookups/review-frequencies');
  },

  listConfidenceLevels(): Promise<AxiosResponse<GlossaryConfidenceLevel[]>> {
    return api.get('/glossary/lookups/confidence-levels');
  },

  listVisibilityLevels(): Promise<AxiosResponse<GlossaryVisibilityLevel[]>> {
    return api.get('/glossary/lookups/visibility-levels');
  },

  listUnitsOfMeasure(): Promise<AxiosResponse<GlossaryUnitOfMeasure[]>> {
    return api.get('/glossary/lookups/units-of-measure');
  },

  listRegulatoryTags(): Promise<AxiosResponse<GlossaryRegulatoryTag[]>> {
    return api.get('/glossary/lookups/regulatory-tags');
  },

  listSubjectAreas(): Promise<AxiosResponse<GlossarySubjectArea[]>> {
    return api.get('/glossary/lookups/subject-areas');
  },

  listLanguages(): Promise<AxiosResponse<GlossaryLanguage[]>> {
    return api.get('/glossary/lookups/languages');
  },

  listClassifications(): Promise<AxiosResponse<DataClassificationRef[]>> {
    return api.get('/data-dictionary/classifications');
  },

  // ----- Junction endpoints (attach/detach) -----

  attachRegulatoryTag(termId: string, tagId: string): Promise<AxiosResponse<void>> {
    return api.post(`/glossary/terms/${termId}/regulatory-tags`, { tag_id: tagId });
  },

  detachRegulatoryTag(termId: string, tagId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/glossary/terms/${termId}/regulatory-tags/${tagId}`);
  },

  attachSubjectArea(termId: string, areaId: string): Promise<AxiosResponse<void>> {
    return api.post(`/glossary/terms/${termId}/subject-areas`, { area_id: areaId });
  },

  detachSubjectArea(termId: string, areaId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/glossary/terms/${termId}/subject-areas/${areaId}`);
  },

  attachTag(termId: string, tagName: string): Promise<AxiosResponse<void>> {
    return api.post(`/glossary/terms/${termId}/tags`, { tag_name: tagName });
  },

  detachTag(termId: string, tagId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/glossary/terms/${termId}/tags/${tagId}`);
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
    return api.get('/stats');
  },
};
