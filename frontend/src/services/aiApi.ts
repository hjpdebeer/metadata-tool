import type { AxiosResponse } from 'axios';
import api from './api';

// --- AI types ---

export interface AiSuggestion {
  suggestion_id: string;
  field_name: string;
  suggested_value: string;
  confidence: number;
  rationale: string;
  source: string;
  model: string | null;
  status: string; // PENDING, ACCEPTED, REJECTED, MODIFIED
  created_at: string;
}

export interface AiEnrichResponse {
  entity_type: string;
  entity_id: string;
  suggestions: AiSuggestion[];
  provider: string;
  model: string;
}

export interface AcceptSuggestionRequest {
  modified_value?: string;
}

export interface FeedbackRequest {
  rating?: number;
  feedback_text?: string;
}

export interface FeedbackResponse {
  feedback_id: string;
  suggestion_id: string;
  message: string;
}

// --- API functions ---

export const aiApi = {
  /**
   * Request AI enrichment for an entity. Calls the AI provider and stores
   * suggestions in PENDING status. All suggestions require human review.
   */
  enrich(
    entityType: string,
    entityId: string,
  ): Promise<AxiosResponse<AiEnrichResponse>> {
    return api.post('/ai/enrich', {
      entity_type: entityType,
      entity_id: entityId,
    });
  },

  /**
   * List all suggestions for a specific entity.
   */
  listSuggestions(
    entityType: string,
    entityId: string,
  ): Promise<AxiosResponse<AiSuggestion[]>> {
    return api.get(`/ai/suggestions/${entityType}/${entityId}`);
  },

  /**
   * Accept a suggestion — applies the value to the entity field.
   * Optionally provide a modified_value to accept with changes.
   */
  acceptSuggestion(
    suggestionId: string,
    modifiedValue?: string,
  ): Promise<AxiosResponse<AiSuggestion>> {
    const body: AcceptSuggestionRequest = {};
    if (modifiedValue !== undefined) {
      body.modified_value = modifiedValue;
    }
    return api.post(`/ai/suggestions/${suggestionId}/accept`, body);
  },

  /**
   * Reject a suggestion.
   */
  rejectSuggestion(
    suggestionId: string,
  ): Promise<AxiosResponse<AiSuggestion>> {
    return api.post(`/ai/suggestions/${suggestionId}/reject`);
  },

  /**
   * Submit feedback (rating + text) for a suggestion.
   */
  submitFeedback(
    suggestionId: string,
    rating?: number,
    feedbackText?: string,
  ): Promise<AxiosResponse<FeedbackResponse>> {
    return api.post(`/ai/suggestions/${suggestionId}/feedback`, {
      rating,
      feedback_text: feedbackText,
    });
  },
};
