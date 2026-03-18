-- Notifications

CREATE TABLE notification_templates (
    template_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_code VARCHAR(64) NOT NULL UNIQUE,
    template_name VARCHAR(256) NOT NULL,
    subject       VARCHAR(512) NOT NULL,
    body_html     TEXT NOT NULL,
    body_text     TEXT NOT NULL,
    description   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO notification_templates (template_code, template_name, subject, body_html, body_text) VALUES
    ('WORKFLOW_TASK_ASSIGNED', 'Task Assigned', 'Action Required: {{entity_type}} "{{entity_name}}" needs your review',
     '<p>Hello {{assignee_name}},</p><p>A new task has been assigned to you:</p><p><strong>{{entity_type}}</strong>: {{entity_name}}<br><strong>Action</strong>: {{action}}<br><strong>Due</strong>: {{due_date}}</p><p><a href="{{task_url}}">Review Now</a></p>',
     'Hello {{assignee_name}}, a new task has been assigned to you: {{entity_type}} "{{entity_name}}". Action: {{action}}. Due: {{due_date}}. Review at: {{task_url}}'),
    ('WORKFLOW_STATE_CHANGED', 'Status Changed', '{{entity_type}} "{{entity_name}}" status changed to {{new_state}}',
     '<p>Hello {{owner_name}},</p><p>The status of <strong>{{entity_type}}</strong> "{{entity_name}}" has changed:</p><p>{{old_state}} → {{new_state}}</p><p>{{comments}}</p><p><a href="{{entity_url}}">View Details</a></p>',
     'Hello {{owner_name}}, the status of {{entity_type}} "{{entity_name}}" has changed from {{old_state}} to {{new_state}}. {{comments}} View at: {{entity_url}}'),
    ('WORKFLOW_SLA_WARNING', 'SLA Warning', 'SLA Warning: {{entity_type}} "{{entity_name}}" review overdue',
     '<p>Hello {{assignee_name}},</p><p>The review of <strong>{{entity_type}}</strong> "{{entity_name}}" is overdue.</p><p><strong>Assigned</strong>: {{assigned_date}}<br><strong>Due</strong>: {{due_date}}</p><p><a href="{{task_url}}">Review Now</a></p>',
     'Hello {{assignee_name}}, the review of {{entity_type}} "{{entity_name}}" is overdue. Due: {{due_date}}. Review at: {{task_url}}');

CREATE TABLE notification_queue (
    notification_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id        UUID REFERENCES notification_templates(template_id),
    recipient_user_id  UUID NOT NULL REFERENCES users(user_id),
    recipient_email    VARCHAR(256) NOT NULL,
    subject            VARCHAR(512) NOT NULL,
    body_html          TEXT NOT NULL,
    body_text          TEXT NOT NULL,
    status             VARCHAR(20) NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','SENDING','SENT','FAILED','CANCELLED')),
    scheduled_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    sent_at            TIMESTAMPTZ,
    error_message      TEXT,
    retry_count        INT NOT NULL DEFAULT 0,
    max_retries        INT NOT NULL DEFAULT 3,
    related_entity_type VARCHAR(50),
    related_entity_id  UUID,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE notification_preferences (
    preference_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users(user_id),
    event_type     VARCHAR(64) NOT NULL,
    email_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    in_app_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, event_type)
);

CREATE TABLE in_app_notifications (
    notification_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(user_id),
    title           VARCHAR(256) NOT NULL,
    message         TEXT NOT NULL,
    link_url        TEXT,
    is_read         BOOLEAN NOT NULL DEFAULT FALSE,
    read_at         TIMESTAMPTZ,
    entity_type     VARCHAR(50),
    entity_id       UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_notification_queue_pending ON notification_queue(status, scheduled_at) WHERE status IN ('PENDING','SENDING');
CREATE INDEX idx_notification_queue_recipient ON notification_queue(recipient_user_id);
CREATE INDEX idx_in_app_notifications_user ON in_app_notifications(user_id);
CREATE INDEX idx_in_app_notifications_unread ON in_app_notifications(user_id, is_read) WHERE is_read = FALSE;
