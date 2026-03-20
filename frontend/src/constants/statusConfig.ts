/**
 * Shared status configuration used across all entity list and detail pages.
 * Single source of truth — do NOT duplicate these maps in individual pages.
 */

export const statusColors: Record<string, string> = {
  DRAFT: 'default',
  PROPOSED: 'processing',
  UNDER_REVIEW: 'warning',
  PENDING_APPROVAL: 'processing',
  REVISED: 'orange',
  ACCEPTED: 'success',
  REJECTED: 'error',
  DEPRECATED: 'default',
  SUPERSEDED: 'default',
};

export const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PROPOSED: 'Proposed',
  UNDER_REVIEW: 'Under Review',
  PENDING_APPROVAL: 'Pending Approval',
  REVISED: 'Revised',
  ACCEPTED: 'Accepted',
  REJECTED: 'Rejected',
  DEPRECATED: 'Deprecated',
  SUPERSEDED: 'Superseded',
};

export const statusOptions = Object.entries(statusLabels)
  .filter(([code]) => !['SUPERSEDED'].includes(code))
  .map(([value, label]) => ({ value, label }));
