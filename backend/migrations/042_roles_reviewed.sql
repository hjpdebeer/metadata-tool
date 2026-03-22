-- Add roles_reviewed flag to track whether an admin has reviewed a user's role assignments.
-- New SSO users auto-provision with DATA_CONSUMER and roles_reviewed = FALSE.
-- Once an admin assigns/removes roles or explicitly confirms, it flips to TRUE.

ALTER TABLE users ADD COLUMN roles_reviewed BOOLEAN NOT NULL DEFAULT TRUE;

-- Mark existing SSO-provisioned users who only have DATA_CONSUMER as unreviewed
UPDATE users u
SET roles_reviewed = FALSE
WHERE u.entra_object_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1 FROM user_roles ur
      JOIN roles r ON r.role_id = ur.role_id
      WHERE ur.user_id = u.user_id
        AND r.role_code != 'DATA_CONSUMER'
  );
