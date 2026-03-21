import type { AxiosResponse } from 'axios';
import api from './api';
import type { BulkUploadResult } from './glossaryApi';

// --- Data Dictionary types ---

export interface DataElement {
  element_id: string;
  element_name: string;
  element_code: string;
  description: string;
  business_definition: string | null;
  business_rules: string | null;
  data_type: string | null;
  max_length: number | null;
  numeric_precision: number | null;
  numeric_scale: number | null;
  format_pattern: string | null;
  allowed_values: string | null;
  default_value: string | null;
  is_nullable: boolean;
  is_cde: boolean;
  cde_rationale: string | null;
  cde_designated_at: string | null;
  glossary_term_id: string | null;
  domain_id: string | null;
  classification_id: string | null;
  status_id: string;
  owner_user_id: string | null;
  steward_user_id: string | null;
  approver_user_id: string | null;
  organisational_unit: string | null;
  review_frequency_id: string | null;
  next_review_date: string | null;
  approved_at: string | null;
  is_pii: boolean;
  version_number: number;
  is_current_version: boolean;
  previous_version_id: string | null;
  created_by: string;
  updated_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface DataElementListItem {
  element_id: string;
  element_name: string;
  element_code: string;
  description: string;
  data_type: string;
  is_cde: boolean;
  domain_name: string | null;
  classification_name: string | null;
  status_code: string;
  status_name: string | null;
  owner_name: string | null;
  glossary_term_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface DataElementFullView extends DataElement {
  glossary_term_name: string | null;
  domain_name: string | null;
  classification_name: string | null;
  owner_name: string | null;
  steward_name: string | null;
  approver_name: string | null;
  review_frequency_name: string | null;
  status_code: string;
  status_name: string | null;
  created_by_name: string | null;
  updated_by_name: string | null;
  workflow_instance_id: string | null;
  technical_columns: TechnicalColumn[];
  quality_rules_count: number;
  linked_processes_count: number;
  linked_applications_count: number;
}

export interface DataClassification {
  classification_id: string;
  classification_code: string;
  classification_name: string;
  description: string | null;
}

export interface SourceSystem {
  system_id: string;
  system_name: string;
  system_code: string;
  system_type: string;
  description: string | null;
  application_id: string | null;
  vendor: string | null;
  environment: string | null;
}

export interface TechnicalSchema {
  schema_id: string;
  system_id: string;
  schema_name: string;
  description: string | null;
}

export interface TechnicalTable {
  table_id: string;
  schema_id: string;
  table_name: string;
  table_type: string;
  description: string | null;
  row_count: number | null;
  size_bytes: number | null;
  is_pii: boolean;
}

export interface TechnicalColumn {
  column_id: string;
  table_id: string;
  column_name: string;
  ordinal_position: number;
  data_type: string | null;
  max_length: number | null;
  numeric_precision: number | null;
  numeric_scale: number | null;
  is_nullable: boolean;
  is_primary_key: boolean;
  is_foreign_key: boolean;
  element_id: string | null;
  element_name: string | null;
  naming_standard_compliant: boolean | null;
  naming_standard_violation: string | null;
}

export interface CreateDataElementRequest {
  element_name: string;
  description: string;
  business_definition?: string;
  business_rules?: string;
  data_type?: string;
  max_length?: number;
  numeric_precision?: number;
  numeric_scale?: number;
  format_pattern?: string;
  allowed_values?: string;
  default_value?: string;
  is_nullable?: boolean;
  glossary_term_id?: string;
  domain_id?: string;
  classification_id?: string;
}

export interface UpdateDataElementRequest {
  element_name?: string;
  element_code?: string;
  description?: string;
  business_definition?: string;
  business_rules?: string;
  data_type?: string;
  max_length?: number;
  numeric_precision?: number;
  numeric_scale?: number;
  format_pattern?: string;
  allowed_values?: string;
  default_value?: string;
  is_nullable?: boolean;
  glossary_term_id?: string;
  domain_id?: string;
  classification_id?: string;
  owner_user_id?: string;
  steward_user_id?: string;
  approver_user_id?: string;
  organisational_unit?: string;
  review_frequency_id?: string;
  is_pii?: boolean;
}

export interface DesignateCdeRequest {
  is_cde: boolean;
  cde_rationale?: string;
}

export interface ListElementsParams {
  query?: string;
  domain_id?: string;
  classification_id?: string;
  is_cde?: boolean;
  status?: string;
  glossary_term_id?: string;
  page?: number;
  page_size?: number;
}

// --- API functions ---

export const dataDictionaryApi = {
  listElements(params: ListElementsParams): Promise<AxiosResponse<DataElementListItem[]>> {
    return api.get('/data-dictionary/elements', { params });
  },

  getElement(id: string): Promise<AxiosResponse<DataElementFullView>> {
    return api.get(`/data-dictionary/elements/${id}`);
  },

  createElement(data: CreateDataElementRequest): Promise<AxiosResponse<DataElement>> {
    return api.post('/data-dictionary/elements', data);
  },

  updateElement(id: string, data: UpdateDataElementRequest): Promise<AxiosResponse<DataElement>> {
    return api.put(`/data-dictionary/elements/${id}`, data);
  },

  listCde(): Promise<AxiosResponse<DataElementListItem[]>> {
    return api.get('/data-dictionary/elements/cde');
  },

  designateCde(id: string, data: DesignateCdeRequest): Promise<AxiosResponse<DataElement>> {
    return api.post(`/data-dictionary/elements/${id}/cde`, data);
  },

  listClassifications(): Promise<AxiosResponse<DataClassification[]>> {
    return api.get('/data-dictionary/classifications');
  },

  listSourceSystems(): Promise<AxiosResponse<SourceSystem[]>> {
    return api.get('/data-dictionary/source-systems');
  },

  listSchemas(systemId: string): Promise<AxiosResponse<TechnicalSchema[]>> {
    return api.get(`/data-dictionary/source-systems/${systemId}/schemas`);
  },

  listTables(schemaId: string): Promise<AxiosResponse<TechnicalTable[]>> {
    return api.get(`/data-dictionary/schemas/${schemaId}/tables`);
  },

  listColumns(tableId: string): Promise<AxiosResponse<TechnicalColumn[]>> {
    return api.get(`/data-dictionary/tables/${tableId}/columns`);
  },

  /** Create a version-based amendment of an accepted data element. */
  amendElement(id: string): Promise<AxiosResponse<DataElement>> {
    return api.post(`/data-dictionary/elements/${id}/amend`);
  },

  /** Discard a draft amendment. Only the creator or admin can discard. */
  discardAmendment(id: string): Promise<AxiosResponse<void>> {
    return api.delete(`/data-dictionary/elements/${id}/discard`);
  },

  /** Download the bulk upload Excel template. */
  downloadBulkUploadTemplate(): Promise<void> {
    return api.get('/data-dictionary/elements/bulk-upload/template', {
      responseType: 'blob',
    }).then((response) => {
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', 'data_element_bulk_upload_template.xlsx');
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
    });
  },

  /** Upload a filled-in bulk upload template. */
  uploadBulkElements(file: File): Promise<AxiosResponse<BulkUploadResult>> {
    const formData = new FormData();
    formData.append('file', file);
    return api.post('/data-dictionary/elements/bulk-upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
      timeout: 120000,
    });
  },
};
