import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the api module — vi.mock is hoisted above imports automatically
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
import { glossaryApi } from '../glossaryApi';

describe('glossaryApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listTerms calls GET /glossary/terms with params', async () => {
    const mockResponse = { data: [] };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    const params = { page: 1, page_size: 20, query: 'test' };
    await glossaryApi.listTerms(params);

    expect(api.get).toHaveBeenCalledWith('/glossary/terms', { params });
  });

  it('getTerm calls GET /glossary/terms/{id}', async () => {
    const mockResponse = { data: { term_id: '123', term_name: 'Test' } };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.getTerm('123');

    expect(api.get).toHaveBeenCalledWith('/glossary/terms/123');
  });

  it('getTermDetail calls GET /glossary/terms/{id}', async () => {
    const mockResponse = { data: { term_id: '456' } };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.getTermDetail('456');

    expect(api.get).toHaveBeenCalledWith('/glossary/terms/456');
  });

  it('createTerm calls POST /glossary/terms', async () => {
    const mockResponse = { data: { term_id: '123' } };
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    const data = { term_name: 'New Term', definition: 'A definition' };
    await glossaryApi.createTerm(data);

    expect(api.post).toHaveBeenCalledWith('/glossary/terms', data);
  });

  it('updateTerm calls PUT /glossary/terms/{id}', async () => {
    const mockResponse = { data: { term_id: '123' } };
    (api.put as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    const data = { term_name: 'Updated Term' };
    await glossaryApi.updateTerm('123', data);

    expect(api.put).toHaveBeenCalledWith('/glossary/terms/123', data);
  });

  it('amendTerm calls POST /glossary/terms/{id}/amend', async () => {
    const mockResponse = { data: { term_id: '123' } };
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.amendTerm('123');

    expect(api.post).toHaveBeenCalledWith('/glossary/terms/123/amend');
  });

  it('discardAmendment calls DELETE /glossary/terms/{id}/discard', async () => {
    const mockResponse = { data: undefined };
    (api.delete as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.discardAmendment('123');

    expect(api.delete).toHaveBeenCalledWith('/glossary/terms/123/discard');
  });

  it('listDomains calls GET /glossary/domains', async () => {
    const mockResponse = { data: [] };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.listDomains();

    expect(api.get).toHaveBeenCalledWith('/glossary/domains');
  });

  it('listCategories calls GET /glossary/categories', async () => {
    const mockResponse = { data: [] };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.listCategories();

    expect(api.get).toHaveBeenCalledWith('/glossary/categories');
  });

  it('attachRegulatoryTag calls POST /glossary/terms/{id}/regulatory-tags', async () => {
    const mockResponse = { data: undefined };
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.attachRegulatoryTag('term-1', 'tag-1');

    expect(api.post).toHaveBeenCalledWith('/glossary/terms/term-1/regulatory-tags', { tag_id: 'tag-1' });
  });

  it('detachRegulatoryTag calls DELETE /glossary/terms/{id}/regulatory-tags/{tagId}', async () => {
    const mockResponse = { data: undefined };
    (api.delete as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse);

    await glossaryApi.detachRegulatoryTag('term-1', 'tag-1');

    expect(api.delete).toHaveBeenCalledWith('/glossary/terms/term-1/regulatory-tags/tag-1');
  });
});
