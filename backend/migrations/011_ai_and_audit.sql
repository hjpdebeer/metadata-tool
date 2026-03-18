-- AI Assistance

CREATE TABLE ai_suggestions (
    suggestion_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type    VARCHAR(50) NOT NULL,
    entity_id      UUID NOT NULL,
    field_name     VARCHAR(128) NOT NULL,
    suggested_value TEXT NOT NULL,
    confidence     NUMERIC(3,2),
    rationale      TEXT,
    source         VARCHAR(20) NOT NULL DEFAULT 'CLAUDE' CHECK(source IN ('CLAUDE','OPENAI')),
    model          VARCHAR(64),
    status         VARCHAR(20) NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','ACCEPTED','REJECTED','MODIFIED')),
    accepted_by    UUID REFERENCES users(user_id),
    accepted_at    TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ai_feedback (
    feedback_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    suggestion_id  UUID NOT NULL REFERENCES ai_suggestions(suggestion_id),
    user_id        UUID NOT NULL REFERENCES users(user_id),
    rating         INT CHECK(rating BETWEEN 1 AND 5),
    feedback_text  TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Audit Trail

CREATE TABLE audit_log (
    audit_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_name   VARCHAR(128) NOT NULL,
    record_id    UUID NOT NULL,
    action       VARCHAR(10) NOT NULL CHECK(action IN ('INSERT','UPDATE','DELETE')),
    old_values   JSONB,
    new_values   JSONB,
    changed_fields TEXT[],
    changed_by   UUID REFERENCES users(user_id),
    changed_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ip_address   INET,
    user_agent   TEXT
);

CREATE TABLE login_audit_log (
    log_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type         VARCHAR(30) NOT NULL,
    user_id            UUID REFERENCES users(user_id),
    attempted_username VARCHAR(256),
    ip_address         INET,
    user_agent         TEXT,
    success            BOOLEAN NOT NULL,
    failure_reason     TEXT,
    occurred_at        TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Generic audit trigger function.
-- Extracts the record_id from the first UUID column found in the row's JSONB
-- representation. Relies on the convention that all tables have a UUID PK
-- column ending in _id as their first column.
CREATE OR REPLACE FUNCTION audit_trigger_function()
RETURNS TRIGGER AS $$
DECLARE
    v_record_id UUID;
    v_changed_by UUID;
    v_row_data JSONB;
    v_pk_col TEXT;
BEGIN
    -- Resolve the PK column name: {singular_table}_id convention
    v_pk_col := regexp_replace(TG_TABLE_NAME, 's$', '') || '_id';

    -- Resolve changed_by from session variable (set by app layer)
    BEGIN
        v_changed_by := current_setting('app.current_user_id', TRUE)::UUID;
    EXCEPTION WHEN OTHERS THEN
        v_changed_by := NULL;
    END;

    IF TG_OP = 'INSERT' THEN
        v_row_data := to_jsonb(NEW);
        v_record_id := (v_row_data ->> v_pk_col)::UUID;
        INSERT INTO audit_log (table_name, record_id, action, new_values, changed_by)
        VALUES (TG_TABLE_NAME, COALESCE(v_record_id, gen_random_uuid()), 'INSERT', v_row_data, v_changed_by);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        v_row_data := to_jsonb(OLD);
        v_record_id := (v_row_data ->> v_pk_col)::UUID;
        INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, changed_by)
        VALUES (TG_TABLE_NAME, COALESCE(v_record_id, gen_random_uuid()), 'UPDATE', v_row_data, to_jsonb(NEW), v_changed_by);
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        v_row_data := to_jsonb(OLD);
        v_record_id := (v_row_data ->> v_pk_col)::UUID;
        INSERT INTO audit_log (table_name, record_id, action, old_values, changed_by)
        VALUES (TG_TABLE_NAME, COALESCE(v_record_id, gen_random_uuid()), 'DELETE', v_row_data, v_changed_by);
        RETURN OLD;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Indexes
CREATE INDEX idx_ai_suggestions_entity ON ai_suggestions(entity_type, entity_id);
CREATE INDEX idx_ai_suggestions_pending ON ai_suggestions(entity_type, entity_id) WHERE status = 'PENDING';
CREATE INDEX idx_audit_log_table ON audit_log(table_name);
CREATE INDEX idx_audit_log_record ON audit_log(table_name, record_id);
CREATE INDEX idx_audit_log_user ON audit_log(changed_by);
CREATE INDEX idx_audit_log_date ON audit_log(changed_at DESC);
CREATE INDEX idx_login_audit_user ON login_audit_log(user_id);
CREATE INDEX idx_login_audit_date ON login_audit_log(occurred_at DESC);
