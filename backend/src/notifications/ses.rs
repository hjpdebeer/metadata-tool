use std::future::Future;
use std::pin::Pin;

use aws_sdk_sesv2::Client as SesClient;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

use super::provider::NotificationProvider;

/// Amazon SES v2 email provider.
///
/// Uses the official AWS SDK which handles:
/// - AWS Signature v4 signing
/// - Credential chain (env vars, ECS task role, EC2 instance profile)
/// - Region configuration
pub struct SesProvider {
    client: SesClient,
    sender: String,
}

impl SesProvider {
    /// Create a new SES provider.
    ///
    /// Loads AWS credentials from the default credential chain (environment
    /// variables, ECS task role via `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI`,
    /// EC2 instance profile, etc.).
    pub async fn new(region: &str, sender: String) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;
        let client = SesClient::new(&config);
        Self { client, sender }
    }
}

impl NotificationProvider for SesProvider {
    fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_html: &str,
        body_text: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>> {
        let to = to.to_string();
        let subject = subject.to_string();
        let body_html = body_html.to_string();
        let body_text = body_text.to_string();

        Box::pin(async move {
            let result = self
                .client
                .send_email()
                .from_email_address(&self.sender)
                .destination(Destination::builder().to_addresses(&to).build())
                .content(
                    EmailContent::builder()
                        .simple(
                            Message::builder()
                                .subject(
                                    Content::builder()
                                        .data(&subject)
                                        .charset("UTF-8")
                                        .build()
                                        .unwrap(),
                                )
                                .body(
                                    Body::builder()
                                        .html(
                                            Content::builder()
                                                .data(&body_html)
                                                .charset("UTF-8")
                                                .build()
                                                .unwrap(),
                                        )
                                        .text(
                                            Content::builder()
                                                .data(&body_text)
                                                .charset("UTF-8")
                                                .build()
                                                .unwrap(),
                                        )
                                        .build(),
                                )
                                .build(),
                        )
                        .build(),
                )
                .send()
                .await
                .map_err(|e| format!("SES send failed: {e}"))?;

            Ok(result.message_id().unwrap_or("unknown").to_string())
        })
    }
}
