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

-- Generic audit trigger function
CREATE OR REPLACE FUNCTION audit_trigger_function()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (table_name, record_id, action, new_values, changed_by)
        VALUES (TG_TABLE_NAME, NEW.*)::TEXT::UUID, 'INSERT', to_jsonb(NEW),
               CASE WHEN TG_TABLE_NAME != 'audit_log' THEN
                   COALESCE(current_setting('app.current_user_id', TRUE)::UUID, NULL)
               END);
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (table_name, record_id, action, old_values, new_values, changed_by)
        VALUES (TG_TABLE_NAME, OLD.*)::TEXT::UUID, 'UPDATE', to_jsonb(OLD), to_jsonb(NEW),
               COALESCE(current_setting('app.current_user_id', TRUE)::UUID, NULL));
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO audit_log (table_name, record_id, action, old_values, changed_by)
        VALUES (TG_TABLE_NAME, OLD.*)::TEXT::UUID, 'DELETE', to_jsonb(OLD),
               COALESCE(current_setting('app.current_user_id', TRUE)::UUID, NULL));
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
