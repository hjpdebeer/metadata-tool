-- Migration 029: Add SUPERSEDED status for replaced term versions
--
-- When a version-based amendment is approved, the old version should be
-- marked SUPERSEDED — a terminal status indicating it was replaced by a
-- newer approved version. SUPERSEDED terms are not visible in the list.

-- Add SUPERSEDED workflow state
INSERT INTO workflow_states (state_id, state_code, state_name, description, is_terminal, display_order)
VALUES (
    'c1d2e3f4-a5b6-7890-cdef-123456789012',
    'SUPERSEDED',
    'Superseded',
    'Replaced by a newer approved version',
    TRUE,
    8
) ON CONFLICT (state_code) DO NOTHING;

-- Add SUPERSEDED entity status
INSERT INTO entity_statuses (status_id, status_code, status_name, description, display_order)
VALUES (
    'd2e3f4a5-b6c7-8901-defa-234567890123',
    'SUPERSEDED',
    'Superseded',
    'Replaced by a newer approved version',
    8
) ON CONFLICT (status_code) DO NOTHING;

-- Mark any old versions that were superseded by amendments
UPDATE glossary_terms
SET status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'SUPERSEDED'),
    updated_at = CURRENT_TIMESTAMP
WHERE is_current_version = FALSE
  AND deleted_at IS NULL
  AND status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'ACCEPTED');
