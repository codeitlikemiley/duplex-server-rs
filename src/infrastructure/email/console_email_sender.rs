use async_trait::async_trait;
use tracing::info;

use super::email_service::{EmailService, EmailMessage, EmailError};

/// Console email sender - logs emails to console (for development/testing)
pub struct ConsoleEmailSender {
    pub enabled: bool,
    pub log_full_content: bool,
}

impl ConsoleEmailSender {
    pub fn new() -> Self {
        Self {
            enabled: true,
            log_full_content: false, // By default, only log summary
        }
    }

    pub fn with_full_content() -> Self {
        Self {
            enabled: true,
            log_full_content: true,
        }
    }
}

impl Default for ConsoleEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailService for ConsoleEmailSender {
    async fn send_email(&self, message: EmailMessage) -> Result<(), EmailError> {
        if !self.enabled {
            return Ok(());
        }

        info!("📧 [CONSOLE EMAIL SENDER] ========================");
        info!("📧 TO: {}", message.to);
        info!("📧 FROM: {}", message.from);
        info!("📧 SUBJECT: {}", message.subject);

        if self.log_full_content {
            info!("📧 BODY (TEXT):\n{}", message.body_text);
            info!("📧 BODY (HTML): [HTML content with {} characters]", message.body_html.len());
        } else {
            // Extract important information like verification tokens or links
            if message.subject.contains("Verify") {
                // Try to extract verification link
                if let Some(url_start) = message.body_text.find("http") {
                    if let Some(url_end) = message.body_text[url_start..].find('\n') {
                        let url = &message.body_text[url_start..url_start + url_end];
                        info!("📧 VERIFICATION LINK: {}", url);
                    }
                }
                // Extract token if it's in the URL
                if let Some(token_pos) = message.body_text.find("token=") {
                    let token_start = token_pos + 6;
                    if let Some(token_end) = message.body_text[token_start..].find(&['\n', '&', ' '][..]) {
                        let token = &message.body_text[token_start..token_start + token_end];
                        info!("📧 VERIFICATION TOKEN: {}", token);
                    }
                }
            }

            if message.subject.contains("Reset") {
                // Try to extract reset link
                if let Some(url_start) = message.body_text.find("http") {
                    if let Some(url_end) = message.body_text[url_start..].find('\n') {
                        let url = &message.body_text[url_start..url_start + url_end];
                        info!("📧 RESET LINK: {}", url);
                    }
                }
            }

            info!("📧 BODY: [Text: {} chars, HTML: {} chars]",
                message.body_text.len(),
                message.body_html.len()
            );
        }

        info!("📧 ================================================");

        Ok(())
    }

    async fn verify_address(&self, email: &str) -> Result<bool, EmailError> {
        // Basic email validation for console sender
        let is_valid = email.contains('@') && email.contains('.') && email.len() > 5;

        if is_valid {
            info!("📧 Email address '{}' is valid", email);
        } else {
            info!("📧 Email address '{}' is invalid", email);
        }

        Ok(is_valid)
    }
}