use std::sync::Arc;

use super::{
    email_service::{EmailService, EmailError},
    console_email_sender::ConsoleEmailSender,
    smtp_email_sender::{SmtpEmailSender, SendGridEmailSender},
};

/// Email service type configuration
#[derive(Debug, Clone)]
pub enum EmailServiceType {
    Console,
    Smtp,
    SendGrid,
    Mock, // For testing
}

impl From<String> for EmailServiceType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "smtp" => EmailServiceType::Smtp,
            "sendgrid" => EmailServiceType::SendGrid,
            "mock" | "test" => EmailServiceType::Mock,
            _ => EmailServiceType::Console,
        }
    }
}

/// Factory for creating email service instances
pub struct EmailServiceFactory;

impl EmailServiceFactory {
    /// Create email service based on environment configuration
    pub fn from_env() -> Result<Arc<dyn EmailService>, EmailError> {
        let service_type = std::env::var("EMAIL_SERVICE")
            .unwrap_or_else(|_| "console".to_string())
            .into();

        Self::create(service_type)
    }

    /// Create email service of specific type
    pub fn create(service_type: EmailServiceType) -> Result<Arc<dyn EmailService>, EmailError> {
        match service_type {
            EmailServiceType::Console => {
                let log_full = std::env::var("EMAIL_LOG_FULL_CONTENT")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false);

                let sender = if log_full {
                    ConsoleEmailSender::with_full_content()
                } else {
                    ConsoleEmailSender::new()
                };

                Ok(Arc::new(sender))
            }
            EmailServiceType::Smtp => {
                let sender = SmtpEmailSender::from_env()?;
                Ok(Arc::new(sender))
            }
            EmailServiceType::SendGrid => {
                let sender = SendGridEmailSender::from_env()?;
                Ok(Arc::new(sender))
            }
            EmailServiceType::Mock => {
                Ok(Arc::new(MockEmailSender::new()))
            }
        }
    }

    /// Create console email service (for development)
    pub fn console() -> Arc<dyn EmailService> {
        Arc::new(ConsoleEmailSender::new())
    }

    /// Create console email service with full content logging
    pub fn console_verbose() -> Arc<dyn EmailService> {
        Arc::new(ConsoleEmailSender::with_full_content())
    }
}

/// Mock email sender for testing
pub struct MockEmailSender {
    pub sent_emails: std::sync::Mutex<Vec<super::EmailMessage>>,
}

impl MockEmailSender {
    pub fn new() -> Self {
        Self {
            sent_emails: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn get_sent_emails(&self) -> Vec<super::EmailMessage> {
        self.sent_emails.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.sent_emails.lock().unwrap().clear();
    }
}

#[async_trait::async_trait]
impl EmailService for MockEmailSender {
    async fn send_email(&self, message: super::EmailMessage) -> Result<(), EmailError> {
        self.sent_emails.lock().unwrap().push(message);
        Ok(())
    }
}