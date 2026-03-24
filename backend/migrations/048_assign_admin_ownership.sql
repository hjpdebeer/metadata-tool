-- Assign the bootstrap admin as owner/steward/approver for all entities
-- that currently have NULL ownership fields.

DO $$
DECLARE
    v_admin_id UUID;
BEGIN
    SELECT user_id INTO v_admin_id FROM users WHERE email = 'admin@metadata-tool.app';

    IF v_admin_id IS NULL THEN
        RAISE NOTICE 'Bootstrap admin not found — skipping ownership assignment';
        RETURN;
    END IF;

    -- Glossary terms
    UPDATE glossary_terms SET owner_user_id = v_admin_id WHERE owner_user_id IS NULL;
    UPDATE glossary_terms SET steward_user_id = v_admin_id WHERE steward_user_id IS NULL;
    UPDATE glossary_terms SET created_by = v_admin_id WHERE created_by IS NULL;

    -- Glossary domains
    UPDATE glossary_domains SET owner_user_id = v_admin_id WHERE owner_user_id IS NULL;

    -- Data elements
    UPDATE data_elements SET owner_user_id = v_admin_id WHERE owner_user_id IS NULL;
    UPDATE data_elements SET steward_user_id = v_admin_id WHERE steward_user_id IS NULL;
    UPDATE data_elements SET approver_user_id = v_admin_id WHERE approver_user_id IS NULL;

    -- Quality rules
    UPDATE quality_rules SET owner_user_id = v_admin_id WHERE owner_user_id IS NULL;

    -- Applications
    UPDATE applications SET business_owner_id = v_admin_id WHERE business_owner_id IS NULL;
    UPDATE applications SET technical_owner_id = v_admin_id WHERE technical_owner_id IS NULL;
    UPDATE applications SET steward_user_id = v_admin_id WHERE steward_user_id IS NULL;
    UPDATE applications SET approver_user_id = v_admin_id WHERE approver_user_id IS NULL;

    -- Business processes
    UPDATE business_processes SET owner_user_id = v_admin_id WHERE owner_user_id IS NULL;

    RAISE NOTICE 'All ownership fields assigned to bootstrap admin';
END $$;
