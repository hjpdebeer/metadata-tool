-- Demo role accounts for testing notifications and workflow.
-- All emails use Protonmail + aliases routing to the same inbox.
-- Password: Password (same as bootstrap admin)

DO $$
DECLARE
    v_user_id UUID;
    v_role_id UUID;
BEGIN
    -- Data Steward
    INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed)
    VALUES ('datasteward', 'hjpdebeer+steward@protonmail.com', 'Demo Data Steward', 'Data Governance', 'Data Steward', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, display_name = EXCLUDED.display_name
    RETURNING user_id INTO v_user_id;
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')) WHERE user_id = v_user_id;
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'DATA_STEWARD';
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (v_user_id, v_role_id, v_user_id) ON CONFLICT DO NOTHING;

    -- Data Owner
    INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed)
    VALUES ('dataowner', 'hjpdebeer+owner@protonmail.com', 'Demo Data Owner', 'Finance', 'Head of Data', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, display_name = EXCLUDED.display_name
    RETURNING user_id INTO v_user_id;
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')) WHERE user_id = v_user_id;
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'DATA_OWNER';
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (v_user_id, v_role_id, v_user_id) ON CONFLICT DO NOTHING;

    -- App Business Owner
    INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed)
    VALUES ('appowner', 'hjpdebeer+appowner@protonmail.com', 'Demo App Business Owner', 'Technology', 'Application Manager', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, display_name = EXCLUDED.display_name
    RETURNING user_id INTO v_user_id;
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')) WHERE user_id = v_user_id;
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'APP_BUSINESS_OWNER';
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (v_user_id, v_role_id, v_user_id) ON CONFLICT DO NOTHING;

    -- Data Producer
    INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed)
    VALUES ('dataproducer', 'hjpdebeer+producer@protonmail.com', 'Demo Data Producer', 'IT Operations', 'ETL Developer', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, display_name = EXCLUDED.display_name
    RETURNING user_id INTO v_user_id;
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')) WHERE user_id = v_user_id;
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'DATA_PRODUCER';
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (v_user_id, v_role_id, v_user_id) ON CONFLICT DO NOTHING;

    -- Data Consumer
    INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed)
    VALUES ('dataconsumer', 'hjpdebeer+consumer@protonmail.com', 'Demo Data Consumer', 'Risk', 'Risk Analyst', TRUE, TRUE)
    ON CONFLICT (email) DO UPDATE SET is_active = TRUE, display_name = EXCLUDED.display_name
    RETURNING user_id INTO v_user_id;
    UPDATE users SET password_hash = crypt('Password', gen_salt('bf')) WHERE user_id = v_user_id;
    SELECT role_id INTO v_role_id FROM roles WHERE role_code = 'DATA_CONSUMER';
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (v_user_id, v_role_id, v_user_id) ON CONFLICT DO NOTHING;

    RAISE NOTICE 'Demo role accounts created (all passwords: Password)';
END $$;
