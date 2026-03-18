-- ============================================================================
-- Migration: 013_dev_auth_seed.sql
-- Purpose: Add password_hash column and seed dev/test users
-- Note: Dev-mode auth (email + password) is automatically disabled when
--        ENTRA_TENANT_ID is configured in the environment.
-- ============================================================================

-- Add password_hash column (nullable — only used in dev mode)
ALTER TABLE users ADD COLUMN password_hash VARCHAR(256);

-- Create a self-referencing admin user first (needed for created_by FK)
-- Then seed test users for each role
-- Password for all seeded users: metadata123

-- Insert admin user (self-referencing created_by handled via deferred constraint or two-step)
DO $$
DECLARE
    admin_id UUID := gen_random_uuid();
    steward_id UUID := gen_random_uuid();
    producer_id UUID := gen_random_uuid();
    consumer_id UUID := gen_random_uuid();
    owner_id UUID := gen_random_uuid();
    app_owner_id UUID := gen_random_uuid();
    process_owner_id UUID := gen_random_uuid();
    pw_hash VARCHAR(256);
    role_admin UUID;
    role_steward UUID;
    role_producer UUID;
    role_consumer UUID;
    role_owner UUID;
    role_app_biz UUID;
    role_process UUID;
BEGIN
    -- Generate bcrypt hash for 'metadata123'
    pw_hash := crypt('metadata123', gen_salt('bf'));

    -- Look up role IDs
    SELECT role_id INTO role_admin FROM roles WHERE role_code = 'ADMIN';
    SELECT role_id INTO role_steward FROM roles WHERE role_code = 'DATA_STEWARD';
    SELECT role_id INTO role_producer FROM roles WHERE role_code = 'DATA_PRODUCER';
    SELECT role_id INTO role_consumer FROM roles WHERE role_code = 'DATA_CONSUMER';
    SELECT role_id INTO role_owner FROM roles WHERE role_code = 'DATA_OWNER';
    SELECT role_id INTO role_app_biz FROM roles WHERE role_code = 'APP_BUSINESS_OWNER';
    SELECT role_id INTO role_process FROM roles WHERE role_code = 'BUSINESS_PROCESS_OWNER';

    -- Insert admin user
    INSERT INTO users (user_id, username, email, display_name, first_name, last_name, department, job_title, is_active, password_hash, created_by)
    VALUES (admin_id, 'admin', 'admin@example.com', 'System Administrator', 'System', 'Admin', 'IT', 'System Administrator', TRUE, pw_hash, admin_id);

    -- Insert test users
    INSERT INTO users (user_id, username, email, display_name, first_name, last_name, department, job_title, is_active, password_hash, created_by)
    VALUES
        (steward_id, 'steward', 'steward@example.com', 'Dana Steward', 'Dana', 'Steward', 'Data Governance', 'Data Steward', TRUE, pw_hash, admin_id),
        (producer_id, 'producer', 'producer@example.com', 'Pat Producer', 'Pat', 'Producer', 'Operations', 'Data Producer', TRUE, pw_hash, admin_id),
        (consumer_id, 'consumer', 'consumer@example.com', 'Chris Consumer', 'Chris', 'Consumer', 'Analytics', 'Data Analyst', TRUE, pw_hash, admin_id),
        (owner_id, 'owner', 'owner@example.com', 'Olivia Owner', 'Olivia', 'Owner', 'Risk Management', 'Data Owner', TRUE, pw_hash, admin_id),
        (app_owner_id, 'appowner', 'appowner@example.com', 'Alex AppOwner', 'Alex', 'AppOwner', 'Technology', 'Application Owner', TRUE, pw_hash, admin_id),
        (process_owner_id, 'processowner', 'processowner@example.com', 'Morgan ProcessOwner', 'Morgan', 'ProcessOwner', 'Operations', 'Process Owner', TRUE, pw_hash, admin_id);

    -- Assign roles
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES
        -- Admin gets ADMIN role
        (admin_id, role_admin, admin_id),
        -- Steward gets DATA_STEWARD
        (steward_id, role_steward, admin_id),
        -- Producer gets DATA_PRODUCER
        (producer_id, role_producer, admin_id),
        -- Consumer gets DATA_CONSUMER
        (consumer_id, role_consumer, admin_id),
        -- Owner gets DATA_OWNER
        (owner_id, role_owner, admin_id),
        -- App owner gets APP_BUSINESS_OWNER
        (app_owner_id, role_app_biz, admin_id),
        -- Process owner gets BUSINESS_PROCESS_OWNER
        (process_owner_id, role_process, admin_id);

    -- Give admin ALL roles for testing convenience
    INSERT INTO user_roles (user_id, role_id, granted_by) VALUES
        (admin_id, role_steward, admin_id),
        (admin_id, role_producer, admin_id),
        (admin_id, role_consumer, admin_id),
        (admin_id, role_owner, admin_id),
        (admin_id, role_app_biz, admin_id),
        (admin_id, role_process, admin_id);
END;
$$;
