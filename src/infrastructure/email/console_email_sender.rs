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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_email_sender_new() {
        let sender = ConsoleEmailSender::new();
        assert!(sender.enabled);
        assert!(!sender.log_full_content);
    }

    #[test]
    fn test_console_email_sender_with_full_content() {
        let sender = ConsoleEmailSender::with_full_content();
        assert!(sender.enabled);
        assert!(sender.log_full_content);
    }

    #[test]
    fn test_console_email_sender_default() {
        let sender = ConsoleEmailSender::default();
        assert!(sender.enabled);
        assert!(!sender.log_full_content);
    }

    #[tokio::test]
    async fn test_send_email_enabled() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<p>Test HTML</p>".to_string(),
            body_text: "Test Text".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_disabled() {
        let sender = ConsoleEmailSender {
            enabled: false,
            log_full_content: false,
        };
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<p>Test HTML</p>".to_string(),
            body_text: "Test Text".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_verification_email() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Please verify</p>".to_string(),
            body_text: "Please verify your email: https://example.com/verify?token=abc123\nThank you.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_password_reset_email() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Reset your password".to_string(),
            body_html: "<p>Reset password</p>".to_string(),
            body_text: "Reset your password: https://example.com/reset?token=def456\nClick the link.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_with_full_content() {
        let sender = ConsoleEmailSender::with_full_content();
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<html><body><p>Test HTML content</p></body></html>".to_string(),
            body_text: "Test text content with details".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_verification_email_without_token() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Please verify</p>".to_string(),
            body_text: "Please verify your email. No link provided.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_verification_email_with_complex_url() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Verify</p>".to_string(),
            body_text: "Click: https://example.com/verify?token=abc123&user=test&redirect=dashboard\nTo verify.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_with_unicode() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "用户@example.com".to_string(),
            from: "系统@example.com".to_string(),
            subject: "测试邮件".to_string(),
            body_html: "<p>测试内容</p>".to_string(),
            body_text: "测试文本内容".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_with_empty_content() {
        let sender = ConsoleEmailSender::new();
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "".to_string(),
            body_html: "".to_string(),
            body_text: "".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_email_with_large_content() {
        let sender = ConsoleEmailSender::new();
        let large_text = "A".repeat(10000);
        let large_html = format!("<p>{}</p>", "B".repeat(10000));

        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Large Content Test".to_string(),
            body_html: large_html,
            body_text: large_text,
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_address_valid() {
        let sender = ConsoleEmailSender::new();

        let valid_emails = vec![
            "test@example.com",
            "user.name@domain.co.uk",
            "user+tag@example.org",
            "user-name@example-domain.com",
            "simple@test.io",
        ];

        for email in valid_emails {
            let result = sender.verify_address(email).await;
            assert!(result.is_ok());
            assert!(result.unwrap(), "Email '{}' should be valid", email);
        }
    }

    #[tokio::test]
    async fn test_verify_address_invalid() {
        let sender = ConsoleEmailSender::new();

        let invalid_emails = vec![
            "notanemail",
            "@example.com",
            "test@",
            "test",
            "test@.com",
            "",
            "a@b", // Too short (less than 5 chars)
        ];

        for email in invalid_emails {
            let result = sender.verify_address(email).await;
            assert!(result.is_ok());
            assert!(!result.unwrap(), "Email '{}' should be invalid", email);
        }
    }

    #[tokio::test]
    async fn test_verify_address_edge_cases() {
        let sender = ConsoleEmailSender::new();

        // Minimum valid email (exactly 5 characters)
        let result = sender.verify_address("a@b.c").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Email with multiple dots
        let result = sender.verify_address("test@sub.domain.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Email with numbers
        let result = sender.verify_address("user123@example123.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_verify_address_unicode() {
        let sender = ConsoleEmailSender::new();

        let unicode_emails = vec![
            "用户@example.com",
            "test@测试.com",
            "пользователь@example.com",
        ];

        for email in unicode_emails {
            let result = sender.verify_address(email).await;
            assert!(result.is_ok());
            // These should be valid as they contain @ and . and are > 5 chars
            assert!(result.unwrap(), "Unicode email '{}' should be valid", email);
        }
    }

    #[tokio::test]
    async fn test_token_extraction_from_url() {
        let sender = ConsoleEmailSender::new();

        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Verify</p>".to_string(),
            body_text: "Verify: https://example.com/verify?token=abc123def456&param=value\nEnd".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_token_extraction_from_url_with_ampersand() {
        let sender = ConsoleEmailSender::new();

        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Verify</p>".to_string(),
            body_text: "Verify: https://example.com/verify?user=test&token=token123&redirect=home".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_url_extraction_multiline() {
        let sender = ConsoleEmailSender::new();

        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Reset your password".to_string(),
            body_html: "<p>Reset</p>".to_string(),
            body_text: "Please reset your password:\nhttps://example.com/reset?token=reset123\nThank you.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_no_url_in_body() {
        let sender = ConsoleEmailSender::new();

        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Verify</p>".to_string(),
            body_text: "Please verify your email address. Contact support if needed.".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_urls_in_body() {
        let sender = ConsoleEmailSender::new();

        let message = EmailMessage {
            to: "user@example.com".to_string(),
            from: "noreply@example.com".to_string(),
            subject: "Verify your email address".to_string(),
            body_html: "<p>Verify</p>".to_string(),
            body_text: "First: https://example.com/verify?token=first\nSecond: https://example.com/verify?token=second".to_string(),
        };

        let result = sender.send_email(message).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_email_sender_configuration() {
        // Test various configurations
        let default_sender = ConsoleEmailSender::default();
        assert!(default_sender.enabled);
        assert!(!default_sender.log_full_content);

        let full_content_sender = ConsoleEmailSender::with_full_content();
        assert!(full_content_sender.enabled);
        assert!(full_content_sender.log_full_content);

        let custom_sender = ConsoleEmailSender {
            enabled: false,
            log_full_content: true,
        };
        assert!(!custom_sender.enabled);
        assert!(custom_sender.log_full_content);
    }
}