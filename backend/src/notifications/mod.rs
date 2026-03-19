// Notification module - handles email queue (via Microsoft Graph API, deferred)
// and in-app notification management.

use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::notifications::InAppNotification;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Internal row types
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct TemplateRow {
    template_id: Uuid,
    subject: String,
    body_html: String,
    body_text: String,
}

#[derive(sqlx::FromRow)]
struct RecipientRow {
    email: String,
}

// ---------------------------------------------------------------------------
// 1. queue_notification — look up template, render, and insert into queue
// ---------------------------------------------------------------------------

/// Queue an email notification by rendering a template with the given variables.
///
/// Looks up the notification template by `template_code`, replaces `{{var}}`
/// placeholders in subject, body_html, and body_text, then inserts a PENDING
/// entry into the `notification_queue` table.
///
/// Actual email sending via Microsoft Graph is deferred — the queue is
/// ready for when the Graph API sender is configured.
pub async fn queue_notification(
    pool: &PgPool,
    recipient_user_id: Uuid,
    template_code: &str,
    variables: &HashMap<String, String>,
    related_entity_type: Option<&str>,
    related_entity_id: Option<Uuid>,
) -> AppResult<()> {
    // Look up the template
    let template = sqlx::query_as::<_, TemplateRow>(
        "SELECT template_id, subject, body_html, body_text
         FROM notification_templates
         WHERE template_code = $1",
    )
    .bind(template_code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "notification template not found: {template_code}"
        ))
    })?;

    // Look up recipient email
    let recipient = sqlx::query_as::<_, RecipientRow>(
        "SELECT email FROM users WHERE user_id = $1",
    )
    .bind(recipient_user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "notification recipient not found: {recipient_user_id}"
        ))
    })?;

    // Render template by replacing {{var}} placeholders
    let subject = render_template(&template.subject, variables);
    let body_html = render_template(&template.body_html, variables);
    let body_text = render_template(&template.body_text, variables);

    // Insert into notification_queue with PENDING status
    sqlx::query(
        "INSERT INTO notification_queue
             (template_id, recipient_user_id, recipient_email, subject,
              body_html, body_text, status, related_entity_type, related_entity_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'PENDING', $7, $8)",
    )
    .bind(template.template_id)
    .bind(recipient_user_id)
    .bind(&recipient.email)
    .bind(&subject)
    .bind(&body_html)
    .bind(&body_text)
    .bind(related_entity_type)
    .bind(related_entity_id)
    .execute(pool)
    .await?;

    tracing::info!(
        recipient_email = %recipient.email,
        template_code = %template_code,
        "Notification queued for delivery"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. queue_workflow_task_notification — convenience wrapper for task assignment
// ---------------------------------------------------------------------------

/// Queue both an email notification and an in-app notification when a
/// workflow task is assigned. Uses the `WORKFLOW_TASK_ASSIGNED` template.
pub async fn queue_workflow_task_notification(
    pool: &PgPool,
    assigned_to_user_id: Uuid,
    entity_type: &str,
    entity_name: &str,
    entity_id: Uuid,
    due_date: Option<&str>,
) -> AppResult<()> {
    // Look up assignee name
    let assignee_name = sqlx::query_scalar::<_, String>(
        "SELECT display_name FROM users WHERE user_id = $1",
    )
    .bind(assigned_to_user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "(unknown)".to_string());

    let mut variables = HashMap::new();
    variables.insert("assignee_name".to_string(), assignee_name.clone());
    variables.insert("entity_type".to_string(), entity_type.to_string());
    variables.insert("entity_name".to_string(), entity_name.to_string());
    variables.insert("action".to_string(), "Review and approve".to_string());
    variables.insert(
        "due_date".to_string(),
        due_date.unwrap_or("Not set").to_string(),
    );
    variables.insert("task_url".to_string(), "/workflow".to_string());

    // Queue email notification
    queue_notification(
        pool,
        assigned_to_user_id,
        "WORKFLOW_TASK_ASSIGNED",
        &variables,
        Some(entity_type),
        Some(entity_id),
    )
    .await?;

    // Also create an in-app notification
    let title = format!("New task: Review {entity_type} \"{entity_name}\"");
    let message = format!(
        "A new review task has been assigned to you for {entity_type} \"{entity_name}\"."
    );

    create_in_app_notification(
        pool,
        assigned_to_user_id,
        &title,
        &message,
        Some("/workflow"),
        Some(entity_type),
        Some(entity_id),
    )
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. queue_workflow_state_changed_notification — notify initiator of state changes
// ---------------------------------------------------------------------------

/// Notify the workflow initiator when the entity reaches a terminal state.
pub async fn queue_workflow_state_changed_notification(
    pool: &PgPool,
    initiator_user_id: Uuid,
    entity_type: &str,
    entity_name: &str,
    entity_id: Uuid,
    old_state: &str,
    new_state: &str,
    comments: Option<&str>,
) -> AppResult<()> {
    // Look up initiator name
    let owner_name = sqlx::query_scalar::<_, String>(
        "SELECT display_name FROM users WHERE user_id = $1",
    )
    .bind(initiator_user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "(unknown)".to_string());

    let mut variables = HashMap::new();
    variables.insert("owner_name".to_string(), owner_name.clone());
    variables.insert("entity_type".to_string(), entity_type.to_string());
    variables.insert("entity_name".to_string(), entity_name.to_string());
    variables.insert("old_state".to_string(), old_state.to_string());
    variables.insert("new_state".to_string(), new_state.to_string());
    variables.insert(
        "comments".to_string(),
        comments.unwrap_or("").to_string(),
    );
    variables.insert("entity_url".to_string(), "/workflow".to_string());

    // Queue email notification
    queue_notification(
        pool,
        initiator_user_id,
        "WORKFLOW_STATE_CHANGED",
        &variables,
        Some(entity_type),
        Some(entity_id),
    )
    .await?;

    // Also create an in-app notification
    let title = format!("{entity_type} \"{entity_name}\" is now {new_state}");
    let message = format!(
        "The status of {entity_type} \"{entity_name}\" has changed from {old_state} to {new_state}."
    );

    create_in_app_notification(
        pool,
        initiator_user_id,
        &title,
        &message,
        Some("/workflow"),
        Some(entity_type),
        Some(entity_id),
    )
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// 4. create_in_app_notification
// ---------------------------------------------------------------------------

/// Insert a new in-app notification for the given user.
pub async fn create_in_app_notification(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    message: &str,
    link_url: Option<&str>,
    entity_type: Option<&str>,
    entity_id: Option<Uuid>,
) -> AppResult<InAppNotification> {
    let notification = sqlx::query_as::<_, InAppNotification>(
        "INSERT INTO in_app_notifications
             (user_id, title, message, link_url, entity_type, entity_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING notification_id, user_id, title, message, link_url,
                   is_read, read_at, entity_type, entity_id, created_at",
    )
    .bind(user_id)
    .bind(title)
    .bind(message)
    .bind(link_url)
    .bind(entity_type)
    .bind(entity_id)
    .fetch_one(pool)
    .await?;

    Ok(notification)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Replace `{{key}}` placeholders in a template string with values from
/// the variables map.
fn render_template(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in variables {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_replaces_placeholders() {
        let template = "Hello {{name}}, your {{entity_type}} \"{{entity_name}}\" is ready.";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("entity_type".to_string(), "Glossary Term".to_string());
        vars.insert("entity_name".to_string(), "Customer".to_string());

        let result = render_template(template, &vars);
        assert_eq!(
            result,
            "Hello Alice, your Glossary Term \"Customer\" is ready."
        );
    }

    #[test]
    fn render_template_leaves_unknown_placeholders() {
        let template = "Hello {{name}}, unknown {{foo}}.";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());

        let result = render_template(template, &vars);
        assert_eq!(result, "Hello Bob, unknown {{foo}}.");
    }
}
