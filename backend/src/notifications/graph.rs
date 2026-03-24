use std::future::Future;
use std::pin::Pin;

use super::provider::NotificationProvider;

/// Microsoft Graph API email provider (stub for future implementation).
///
/// When implemented, this will use the Graph API `/me/sendMail` endpoint
/// with OAuth2 client credentials flow to send emails via a configured
/// Microsoft 365 mailbox.
pub struct GraphProvider {
    _tenant_id: String,
    _client_id: String,
    _client_secret: String,
    _sender_email: String,
}

impl GraphProvider {
    pub fn new(
        tenant_id: String,
        client_id: String,
        client_secret: String,
        sender_email: String,
    ) -> Self {
        Self {
            _tenant_id: tenant_id,
            _client_id: client_id,
            _client_secret: client_secret,
            _sender_email: sender_email,
        }
    }
}

impl NotificationProvider for GraphProvider {
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
            tracing::warn!(
                to = to.as_str(),
                subject = subject.as_str(),
                "Graph API email sending not yet implemented — notification skipped"
            );
            Ok("graph-not-implemented".to_string())
        })
    }
}
