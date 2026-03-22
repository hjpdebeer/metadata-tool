-- Fix: migration 045 failed due to username conflict.
-- Create bootstrap admin with unique username.

DO $$
DECLARE
    v_admin_id UUID;
    v_role_id UUID;
BEGIN
    -- Check if bootstrap admin already exists
    SELECT user_id INTO v_admin_id FROM users WHERE email = 'admin@metadata-tool.app';

    IF v_admin_id IS NULL THEN
        INSERT INTO users (username, email, display_name, is_active, roles_reviewed)
        VALUES ('sysadmin', 'admin@metadata-tool.app', 'System Administrator', TRUE, TRUE)
        RETURNING user_id INTO v_admin_id;
    END IF;

    -- Set password hash using pgcrypto bcrypt
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')), is_active = TRUE
    WHERE user_id = v_admin_id;

    -- Assign ADMIN role if not already assigned
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'ADMIN';

    INSERT INTO user_roles (user_id, role_id, granted_by, effective_from)
    VALUES (v_admin_id, v_role_id, v_admin_id, NOW())
    ON CONFLICT DO NOTHING;
END $$;
