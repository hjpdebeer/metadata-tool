import type { AxiosResponse } from 'axios';
import api from './api';
import type { BulkUploadResult } from './glossaryApi';

// --- Application Classification types ---

export interface ApplicationClassification {
  classification_id: string;
  classification_code: string;
  classification_name: string;
  description: string | null;
  display_order: number;
}

// --- New lookup types ---

export interface DisasterRecoveryTier {
  dr_tier_id: string;
  tier_code: string;
  tier_name: string;
  rto_hours: number;
  rpo_minutes: number;
  description: string | null;
}

export interface ApplicationLifecycleStage {
  stage_id: string;
  stage_code: string;
  stage_name: string;
  description: string | null;
}

export interface ApplicationCriticalityTier {
  tier_id: string;
  tier_code: string;
  tier_name: string;
  description: string | null;
}

export interface ApplicationRiskRating {
  rating_id: string;
  rating_code: string;
  rating_name: string;
  description: string | null;
}

// --- Application types ---

export interface Application {
  application_id: string;
  application_name: string;
  application_code: string;
  description: string;
  // Classification & type
  classification_id: string | null;
  deployment_type: string | null;
  technology_stack: unknown | null;
  // Ownership & governance
  status_id: string;
  business_owner_id: string | null;
  technical_owner_id: string | null;
  steward_user_id: string | null;
  approver_user_id: string | null;
  organisational_unit: string | null;
  // Vendor & product
  vendor: string | null;
  vendor_product_name: string | null;
  version: string | null;
  license_type: string | null;
  // Business context
  abbreviation: string | null;
  external_reference_id: string | null;
  business_capability: string | null;
  user_base: string | null;
  // Criticality & risk
  is_cba: boolean;
  cba_rationale: string | null;
  criticality_tier_id: string | null;
  risk_rating_id: string | null;
  // Compliance
  data_classification_id: string | null;
  regulatory_scope: string | null;
  last_security_assessment: string | null;
  // Operational
  support_model: string | null;
  dr_tier_id: string | null;
  // Lifecycle
  lifecycle_stage_id: string | null;
  go_live_date: string | null;
  retirement_date: string | null;
  contract_end_date: string | null;
  review_frequency_id: string | null;
  next_review_date: string | null;
  approved_at: string | null;
  // Reference
  documentation_url: string | null;
  // Versioning
  version_number: number;
  is_current_version: boolean;
  previous_version_id: string | null;
  // Audit
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
  abbreviation: string | null;
  classification_name: string | null;
  status_code: string;
  status_name: string;
  business_owner_name: string | null;
  technical_owner_name: string | null;
  vendor: string | null;
  is_cba: boolean;
  deployment_type: string | null;
  lifecycle_stage_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface ApplicationFullView {
  // Entity columns
  application_id: string;
  application_name: string;
  application_code: string;
  description: string;
  classification_id: string | null;
  classification_name: string | null;
  deployment_type: string | null;
  technology_stack: unknown | null;
  status_id: string;
  status_code: string | null;
  business_owner_id: string | null;
  business_owner_name: string | null;
  technical_owner_id: string | null;
  technical_owner_name: string | null;
  steward_user_id: string | null;
  steward_name: string | null;
  approver_user_id: string | null;
  approver_name: string | null;
  organisational_unit: string | null;
  vendor: string | null;
  vendor_product_name: string | null;
  version: string | null;
  license_type: string | null;
  abbreviation: string | null;
  external_reference_id: string | null;
  business_capability: string | null;
  user_base: string | null;
  is_cba: boolean;
  cba_rationale: string | null;
  criticality_tier_id: string | null;
  criticality_tier_name: string | null;
  risk_rating_id: string | null;
  risk_rating_name: string | null;
  data_classification_id: string | null;
  data_classification_name: string | null;
  regulatory_scope: string | null;
  last_security_assessment: string | null;
  support_model: string | null;
  dr_tier_id: string | null;
  dr_tier_name: string | null;
  dr_tier_rto_hours: number | null;
  dr_tier_rpo_minutes: number | null;
  lifecycle_stage_id: string | null;
  lifecycle_stage_name: string | null;
  go_live_date: string | null;
  retirement_date: string | null;
  contract_end_date: string | null;
  review_frequency_id: string | null;
  review_frequency_name: string | null;
  next_review_date: string | null;
  approved_at: string | null;
  documentation_url: string | null;
  version_number: number;
  is_current_version: boolean;
  previous_version_id: string | null;
  created_by: string;
  created_by_name: string | null;
  updated_by: string | null;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
  // Junction data
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
  description: string;
  classification_id?: string;
  vendor?: string;
  vendor_product_name?: string;
  version?: string;
  deployment_type?: string;
  technology_stack?: string;
  is_cba?: boolean;
  cba_rationale?: string;
  go_live_date?: string;
  documentation_url?: string;
  abbreviation?: string;
  external_reference_id?: string;
  license_type?: string;
  lifecycle_stage_id?: string;
}

export interface UpdateApplicationRequest {
  application_name?: string;
  description?: string;
  classification_id?: string;
  vendor?: string;
  vendor_product_name?: string;
  version?: string;
  deployment_type?: string;
  technology_stack?: string;
  is_cba?: boolean;
  cba_rationale?: string;
  go_live_date?: string;
  retirement_date?: string;
  documentation_url?: string;
  abbreviation?: string;
  external_reference_id?: string;
  business_capability?: string;
  user_base?: string;
  license_type?: string;
  lifecycle_stage_id?: string;
  criticality_tier_id?: string;
  risk_rating_id?: string;
  data_classification_id?: string;
  regulatory_scope?: string;
  last_security_assessment?: string;
  support_model?: string;
  dr_tier_id?: string;
  contract_end_date?: string;
  review_frequency_id?: string;
  // Ownership
  business_owner_id?: string;
  technical_owner_id?: string;
  steward_user_id?: string;
  approver_user_id?: string;
  organisational_unit?: string;
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
  is_cba?: boolean;
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

  /** Propose an amendment to an accepted application. Creates a new version in DRAFT. */
  amendApplication(id: string): Promise<AxiosResponse<Application>> {
    return api.post(`/applications/${id}/amend`);
  },

  /** Discard a draft amendment. Only the creator or admin can discard. */
  discardAmendment(id: string): Promise<AxiosResponse<void>> {
    return api.delete(`/applications/${id}/discard`);
  },

  listClassifications(): Promise<AxiosResponse<ApplicationClassification[]>> {
    return api.get('/applications/classifications');
  },

  listDrTiers(): Promise<AxiosResponse<DisasterRecoveryTier[]>> {
    return api.get('/applications/dr-tiers');
  },

  listLifecycleStages(): Promise<AxiosResponse<ApplicationLifecycleStage[]>> {
    return api.get('/applications/lifecycle-stages');
  },

  listCriticalityTiers(): Promise<AxiosResponse<ApplicationCriticalityTier[]>> {
    return api.get('/applications/criticality-tiers');
  },

  listRiskRatings(): Promise<AxiosResponse<ApplicationRiskRating[]>> {
    return api.get('/applications/risk-ratings');
  },

  linkDataElement(appId: string, data: LinkDataElementRequest): Promise<AxiosResponse<ApplicationDataElementLink>> {
    return api.post(`/applications/${appId}/elements`, data);
  },

  listAppElements(appId: string): Promise<AxiosResponse<ApplicationDataElementLink[]>> {
    return api.get(`/applications/${appId}/elements`);
  },

  listInterfaces(appId: string): Promise<AxiosResponse<ApplicationInterface[]>> {
    return api.get(`/applications/${appId}/interfaces`);
  },

  // ----- Bulk upload -----

  downloadBulkUploadTemplate(): Promise<void> {
    return api.get('/applications/bulk-upload/template', {
      responseType: 'blob',
    }).then((response) => {
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', 'application_bulk_upload_template.xlsx');
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
    });
  },

  uploadBulkApplications(file: File): Promise<AxiosResponse<BulkUploadResult>> {
    const formData = new FormData();
    formData.append('file', file);
    return api.post('/applications/bulk-upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
      timeout: 120000,
    });
  },
};
