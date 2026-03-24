use std::future::Future;
use std::pin::Pin;

use super::provider::NotificationProvider;

/// No-op email provider used when email notifications are disabled.
///
/// Logs a debug message and returns immediately. This is the default
/// provider when `NOTIFICATION_PROVIDER` is unset or set to "disabled".
pub struct DisabledProvider;

impl NotificationProvider for DisabledProvider {
    fn send_email(
        &self,
        to: &str,
        subject: &str,
        _body_html: &str,
        _body_text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
        let to = to.to_string();
        let subject = subject.to_string();

        Box::pin(async move {
            tracing::debug!(
                to = to.as_str(),
                subject = subject.as_str(),
                "Email notifications disabled — skipping"
            );
            Ok("disabled".to_string())
        })
    }
}
