use std::future::Future;
use std::pin::Pin;

/// Trait for email notification providers (SES, Graph API, etc.)
///
/// Returns a boxed future to support dynamic dispatch (`dyn NotificationProvider`),
/// which allows runtime selection of the email provider based on configuration.
pub trait NotificationProvider: Send + Sync {
    fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_html: &str,
        body_text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>>;
}
