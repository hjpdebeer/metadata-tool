-- Remove all users except the bootstrap admin (admin@metadata-tool.app).
-- Strategy: reassign all NOT NULL user FKs to admin, nullify all nullable ones,
-- then delete from user-only tables, then delete users.

DO $$
DECLARE
    v_admin_id UUID;
    v_sql TEXT;
    v_rec RECORD;
    v_user_only_tables TEXT[] := ARRAY[
        'user_roles', 'sso_sessions', 'in_app_notifications',
        'notification_preferences', 'notification_queue',
        'ai_feedback', 'workflow_history', 'workflow_tasks'
    ];
    v_tbl TEXT;
BEGIN
    SELECT user_id INTO v_admin_id FROM users WHERE email = 'admin@metadata-tool.app';

    IF v_admin_id IS NULL THEN
        RAISE NOTICE 'Bootstrap admin not found — skipping user cleanup';
        RETURN;
    END IF;

    -- Step 1: Delete from user-centric tables (these are safe to delete rows from)
    FOREACH v_tbl IN ARRAY v_user_only_tables LOOP
        FOR v_rec IN
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
            JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name
            WHERE ccu.table_name = 'users' AND ccu.column_name = 'user_id'
              AND tc.constraint_type = 'FOREIGN KEY'
              AND kcu.table_name = v_tbl
        LOOP
            v_sql := format('DELETE FROM %I WHERE %I != %L', v_tbl, v_rec.column_name, v_admin_id);
            EXECUTE v_sql;
        END LOOP;
    END LOOP;

    -- Step 2: For ALL other tables with user FKs, reassign NOT NULL columns to admin
    -- and nullify nullable columns
    FOR v_rec IN
        SELECT kcu.table_name, kcu.column_name, c.is_nullable
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
        JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name
        JOIN information_schema.columns c ON c.table_name = kcu.table_name AND c.column_name = kcu.column_name
        WHERE ccu.table_name = 'users' AND ccu.column_name = 'user_id'
          AND tc.constraint_type = 'FOREIGN KEY'
          AND kcu.table_name != 'users'
          AND NOT (kcu.table_name = ANY(v_user_only_tables))
    LOOP
        IF v_rec.is_nullable = 'YES' THEN
            v_sql := format('UPDATE %I SET %I = NULL WHERE %I IS NOT NULL AND %I != %L',
                            v_rec.table_name, v_rec.column_name, v_rec.column_name, v_rec.column_name, v_admin_id);
        ELSE
            v_sql := format('UPDATE %I SET %I = %L WHERE %I != %L',
                            v_rec.table_name, v_rec.column_name, v_admin_id, v_rec.column_name, v_admin_id);
        END IF;
        EXECUTE v_sql;
    END LOOP;

    -- Step 3: Delete all non-admin users
    DELETE FROM users WHERE user_id != v_admin_id;

    RAISE NOTICE 'All users removed except bootstrap admin (admin@metadata-tool.app)';
END $$;
