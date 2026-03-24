import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../api', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() },
    },
  },
}));

import api from '../api';
import { dataQualityApi } from '../dataQualityApi';

describe('dataQualityApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listDimensions calls GET /data-quality/dimensions', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.listDimensions();

    expect(api.get).toHaveBeenCalledWith('/data-quality/dimensions');
  });

  it('listRuleTypes calls GET /data-quality/rule-types', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.listRuleTypes();

    expect(api.get).toHaveBeenCalledWith('/data-quality/rule-types');
  });

  it('listRules calls GET /data-quality/rules with params', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    const params = { page: 1, page_size: 10, severity: 'HIGH' as const };
    await dataQualityApi.listRules(params);

    expect(api.get).toHaveBeenCalledWith('/data-quality/rules', { params });
  });

  it('getRule calls GET /data-quality/rules/{id}', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await dataQualityApi.getRule('rule-123');

    expect(api.get).toHaveBeenCalledWith('/data-quality/rules/rule-123');
  });

  it('createRule calls POST /data-quality/rules', async () => {
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    const data = {
      rule_name: 'Test Rule',
      rule_code: 'TEST_RULE',
      description: 'A test rule',
      dimension_id: 'dim-1',
      rule_type_id: 'type-1',
      rule_definition: { check: 'not_null' },
      severity: 'HIGH',
    };
    await dataQualityApi.createRule(data);

    expect(api.post).toHaveBeenCalledWith('/data-quality/rules', data);
  });

  it('updateRule calls PUT /data-quality/rules/{id}', async () => {
    (api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    const data = { rule_name: 'Updated Rule' };
    await dataQualityApi.updateRule('rule-123', data);

    expect(api.put).toHaveBeenCalledWith('/data-quality/rules/rule-123', data);
  });

  it('deleteRule calls DELETE /data-quality/rules/{id}', async () => {
    (api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await dataQualityApi.deleteRule('rule-123');

    expect(api.delete).toHaveBeenCalledWith('/data-quality/rules/rule-123');
  });

  it('getAssessments calls GET /data-quality/rules/{id}/assessments', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.getAssessments('rule-123');

    expect(api.get).toHaveBeenCalledWith('/data-quality/rules/rule-123/assessments');
  });

  it('createAssessment calls POST /data-quality/assessments', async () => {
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    const data = {
      rule_id: '123',
      records_assessed: 100,
      records_passed: 95,
      records_failed: 5,
    };
    await dataQualityApi.createAssessment(data);

    expect(api.post).toHaveBeenCalledWith('/data-quality/assessments', data);
  });

  it('getRecentAssessments calls GET /data-quality/assessments/recent with limit', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.getRecentAssessments(5);

    expect(api.get).toHaveBeenCalledWith('/data-quality/assessments/recent', { params: { limit: 5 } });
  });

  it('getRecentAssessments defaults to limit 10', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.getRecentAssessments();

    expect(api.get).toHaveBeenCalledWith('/data-quality/assessments/recent', { params: { limit: 10 } });
  });

  it('getElementScores calls GET /data-quality/scores/element/{id}', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await dataQualityApi.getElementScores('elem-123');

    expect(api.get).toHaveBeenCalledWith('/data-quality/scores/element/elem-123');
  });

  it('suggestQualityRules calls POST /ai/suggest-quality-rules', async () => {
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await dataQualityApi.suggestQualityRules('elem-123');

    expect(api.post).toHaveBeenCalledWith('/ai/suggest-quality-rules', { element_id: 'elem-123' });
  });
});
