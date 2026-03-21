import type { AxiosResponse } from 'axios';
import api from './api';

// --- Quality Dimension types ---

export interface QualityDimension {
  dimension_id: string;
  dimension_code: string;
  dimension_name: string;
  description: string | null;
}

export interface QualityDimensionSummary extends QualityDimension {
  rules_count: number;
  avg_score: number | null;
  last_assessed_at: string | null;
}

// --- Quality Rule Type types ---

export interface QualityRuleType {
  rule_type_id: string;
  type_code: string;
  type_name: string;
  description: string | null;
}

// --- Quality Rule types ---

export interface QualityRule {
  rule_id: string;
  rule_name: string;
  rule_code: string;
  description: string;
  dimension_id: string;
  rule_type_id: string;
  element_id: string | null;
  column_id: string | null;
  rule_definition: Record<string, unknown>;
  threshold_percentage: number;
  severity: string;
  is_active: boolean;
  status_id: string;
  owner_user_id: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
  workflow_instance_id: string | null;
  // Resolved lookup names (from JOINs in detail query)
  dimension_name: string;
  rule_type_name: string;
  element_name: string | null;
}

export interface QualityRuleListItem {
  rule_id: string;
  rule_name: string;
  rule_code: string;
  description: string;
  dimension_name: string;
  dimension_code: string;
  rule_type_name: string;
  element_name: string | null;
  severity: string;
  is_active: boolean;
  status_code: string;
  status_name: string | null;
  owner_name: string | null;
  threshold_percentage: number;
  created_at: string;
  updated_at: string;
}

// --- Quality Assessment types ---

export interface QualityAssessment {
  assessment_id: string;
  rule_id: string;
  assessed_at: string;
  records_assessed: number;
  records_passed: number;
  records_failed: number;
  score_percentage: number;
  status: string;
  details: string | null;
}

// --- Quality Score types ---

export interface QualityScore {
  score_id: string;
  element_id: string | null;
  table_id: string | null;
  dimension_id: string | null;
  overall_score: number;
  period_start: string;
  period_end: string;
}

// --- Request types ---

export interface CreateQualityRuleRequest {
  rule_name: string;
  rule_code: string;
  description: string;
  dimension_id: string;
  rule_type_id: string;
  element_id?: string;
  column_id?: string;
  rule_definition: Record<string, unknown>;
  threshold_percentage?: number;
  severity: string;
  is_active?: boolean;
}

export interface UpdateQualityRuleRequest {
  rule_name?: string;
  rule_code?: string;
  description?: string;
  dimension_id?: string;
  rule_type_id?: string;
  element_id?: string;
  column_id?: string;
  rule_definition?: Record<string, unknown>;
  threshold_percentage?: number;
  severity?: string;
  is_active?: boolean;
}

export interface CreateAssessmentRequest {
  rule_id: string;
  records_assessed: number;
  records_passed: number;
  records_failed: number;
  details?: string;
}

export interface ListRulesParams {
  query?: string;
  dimension_id?: string;
  element_id?: string;
  severity?: string;
  is_active?: boolean;
  status?: string;
  page?: number;
  page_size?: number;
}

// --- API functions ---

export const dataQualityApi = {
  listDimensions(): Promise<AxiosResponse<QualityDimensionSummary[]>> {
    return api.get('/data-quality/dimensions');
  },

  listRuleTypes(): Promise<AxiosResponse<QualityRuleType[]>> {
    return api.get('/data-quality/rule-types');
  },

  listRules(params: ListRulesParams): Promise<AxiosResponse<QualityRuleListItem[]>> {
    return api.get('/data-quality/rules', { params });
  },

  getRule(id: string): Promise<AxiosResponse<QualityRule>> {
    return api.get(`/data-quality/rules/${id}`);
  },

  createRule(data: CreateQualityRuleRequest): Promise<AxiosResponse<QualityRule>> {
    return api.post('/data-quality/rules', data);
  },

  updateRule(id: string, data: UpdateQualityRuleRequest): Promise<AxiosResponse<QualityRule>> {
    return api.put(`/data-quality/rules/${id}`, data);
  },

  getAssessments(ruleId: string): Promise<AxiosResponse<QualityAssessment[]>> {
    return api.get(`/data-quality/rules/${ruleId}/assessments`);
  },

  createAssessment(data: CreateAssessmentRequest): Promise<AxiosResponse<QualityAssessment>> {
    return api.post('/data-quality/assessments', data);
  },

  getElementScores(elementId: string): Promise<AxiosResponse<QualityScore[]>> {
    return api.get(`/data-quality/scores/element/${elementId}`);
  },

  getRecentAssessments(limit?: number): Promise<AxiosResponse<QualityAssessment[]>> {
    return api.get('/data-quality/assessments/recent', { params: { limit: limit || 10 } });
  },
};
