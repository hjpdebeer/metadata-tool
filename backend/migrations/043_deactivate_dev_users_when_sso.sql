-- Deactivate dev-mode seed users when SSO is configured.
-- These users cannot login when Entra is active anyway (403 Forbidden),
-- but their ADMIN role prevents the first-SSO-user-is-admin logic from working.
-- This migration removes their role assignments so the first real SSO user gets ADMIN.

DELETE FROM user_roles
WHERE user_id IN (
    SELECT user_id FROM users
    WHERE email LIKE '%@example.com'
);

UPDATE users
SET is_active = FALSE
WHERE email LIKE '%@example.com';
