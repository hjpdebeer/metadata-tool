use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use super::provider::NotificationProvider;

#[derive(sqlx::FromRow)]
struct QueuedNotification {
    notification_id: Uuid,
    recipient_email: String,
    subject: String,
    body_html: String,
    body_text: String,
    retry_count: i32,
    max_retries: i32,
}

/// Spawn the background email processor task.
///
/// Polls the `notification_queue` table every 30 seconds for PENDING
/// notifications, claims a batch of up to 10 via `FOR UPDATE SKIP LOCKED`,
/// and dispatches them through the configured [`NotificationProvider`].
///
/// Failed sends are retried with exponential backoff (1 min, 5 min, 15 min)
/// until `max_retries` is reached, at which point the notification is marked
/// FAILED.
pub fn spawn(pool: PgPool, provider: Arc<dyn NotificationProvider>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = process_batch(&pool, &provider).await {
                tracing::error!(error = %e, "notification processor error");
            }
        }
    });
    tracing::info!("Notification email processor started (polling every 30s)");
}

async fn process_batch(
    pool: &PgPool,
    provider: &Arc<dyn NotificationProvider>,
) -> Result<(), sqlx::Error> {
    // Claim a batch of pending notifications atomically
    let notifications = sqlx::query_as::<_, QueuedNotification>(
        r#"
        UPDATE notification_queue
        SET status = 'SENDING'
        WHERE notification_id IN (
            SELECT notification_id
            FROM notification_queue
            WHERE status = 'PENDING'
              AND scheduled_at <= CURRENT_TIMESTAMP
              AND retry_count < max_retries
            ORDER BY scheduled_at ASC
            LIMIT 10
            FOR UPDATE SKIP LOCKED
        )
        RETURNING notification_id, recipient_email, subject, body_html, body_text,
                  retry_count, max_retries
        "#,
    )
    .fetch_all(pool)
    .await?;

    for notif in notifications {
        let result = provider
            .send_email(
                &notif.recipient_email,
                &notif.subject,
                &notif.body_html,
                &notif.body_text,
            )
            .await;

        match result {
            Ok(message_id) => {
                sqlx::query(
                    "UPDATE notification_queue \
                     SET status = 'SENT', sent_at = CURRENT_TIMESTAMP, error_message = $2 \
                     WHERE notification_id = $1",
                )
                .bind(notif.notification_id)
                .bind(format!("message_id: {message_id}"))
                .execute(pool)
                .await?;

                tracing::info!(
                    notification_id = %notif.notification_id,
                    to = %notif.recipient_email,
                    "Email sent successfully"
                );
            }
            Err(error) => {
                let new_retry = notif.retry_count + 1;
                if new_retry >= notif.max_retries {
                    sqlx::query(
                        "UPDATE notification_queue \
                         SET status = 'FAILED', retry_count = $2, error_message = $3 \
                         WHERE notification_id = $1",
                    )
                    .bind(notif.notification_id)
                    .bind(new_retry)
                    .bind(&error)
                    .execute(pool)
                    .await?;

                    tracing::error!(
                        notification_id = %notif.notification_id,
                        to = %notif.recipient_email,
                        error = %error,
                        retries = new_retry,
                        "Email permanently failed after max retries"
                    );
                } else {
                    // Exponential backoff: 1 min, 5 min, 15 min
                    let backoff_minutes = match new_retry {
                        1 => 1,
                        2 => 5,
                        _ => 15,
                    };
                    sqlx::query(
                        "UPDATE notification_queue \
                         SET status = 'PENDING', retry_count = $2, error_message = $3, \
                             scheduled_at = CURRENT_TIMESTAMP + ($4 || ' minutes')::INTERVAL \
                         WHERE notification_id = $1",
                    )
                    .bind(notif.notification_id)
                    .bind(new_retry)
                    .bind(&error)
                    .bind(backoff_minutes.to_string())
                    .execute(pool)
                    .await?;

                    tracing::warn!(
                        notification_id = %notif.notification_id,
                        retry = new_retry,
                        next_attempt_minutes = backoff_minutes,
                        "Email send failed, will retry"
                    );
                }
            }
        }
    }

    Ok(())
}
