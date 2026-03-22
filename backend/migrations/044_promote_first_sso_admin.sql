-- Promote the first SSO user to ADMIN if no admin exists.
-- This handles the case where the user was already provisioned as DATA_CONSUMER
-- before the first-user-is-admin logic was deployed.

INSERT INTO user_roles (user_id, role_id, granted_by, effective_from)
SELECT u.user_id, r.role_id, u.user_id, NOW()
FROM users u
CROSS JOIN roles r
WHERE r.role_code = 'ADMIN'
  AND u.entra_object_id IS NOT NULL
  AND u.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1 FROM user_roles ur2
      JOIN roles r2 ON r2.role_id = ur2.role_id
      WHERE r2.role_code = 'ADMIN'
  )
ORDER BY u.created_at ASC
LIMIT 1;
