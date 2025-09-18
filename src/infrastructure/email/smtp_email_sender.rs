use async_trait::async_trait;
use tracing::{info, error};

use super::email_service::{EmailService, EmailMessage, EmailError};

/// SMTP configuration
#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
}

impl SmtpConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, EmailError> {
        Ok(Self {
            host: std::env::var("SMTP_HOST")
                .map_err(|_| EmailError::ConfigurationError("SMTP_HOST not set".to_string()))?,
            port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .map_err(|_| EmailError::ConfigurationError("Invalid SMTP_PORT".to_string()))?,
            username: std::env::var("SMTP_USERNAME")
                .map_err(|_| EmailError::ConfigurationError("SMTP_USERNAME not set".to_string()))?,
            password: std::env::var("SMTP_PASSWORD")
                .map_err(|_| EmailError::ConfigurationError("SMTP_PASSWORD not set".to_string()))?,
            from_email: std::env::var("SMTP_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@quake.app".to_string()),
            from_name: std::env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "Quake App".to_string()),
            use_tls: std::env::var("SMTP_USE_TLS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        })
    }
}

/// SMTP email sender for production use
///
/// This is a placeholder implementation. In production, you would use:
/// - `lettre` crate for SMTP
/// - `sendgrid` crate for SendGrid API
/// - `rusoto_ses` for AWS SES
/// - `mailgun-rs` for Mailgun API
pub struct SmtpEmailSender {
    config: SmtpConfig,
}

impl SmtpEmailSender {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Result<Self, EmailError> {
        let config = SmtpConfig::from_env()?;
        Ok(Self::new(config))
    }
}

#[async_trait]
impl EmailService for SmtpEmailSender {
    async fn send_email(&self, message: EmailMessage) -> Result<(), EmailError> {
        use lettre::{
            AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
            transport::smtp::authentication::Credentials,
        };

        info!(
            "📧 [SMTP] Sending email to {} with subject: {}",
            message.to, message.subject
        );

        // Build the email message
        let email = Message::builder()
            .from(format!("{} <{}>", self.config.from_name, self.config.from_email)
                .parse()
                .map_err(|e| EmailError::ConfigurationError(format!("Invalid from address: {}", e)))?)
            .to(message.to.parse()
                .map_err(|e| EmailError::InvalidAddress(format!("Invalid to address: {}", e)))?)
            .subject(message.subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::plain(message.body_text)
                    )
                    .singlepart(
                        lettre::message::SinglePart::html(message.body_html)
                    )
            )
            .map_err(|e| EmailError::SendFailed(format!("Failed to build email: {}", e)))?;

        // Set up SMTP credentials
        let creds = Credentials::new(
            self.config.username.clone(),
            self.config.password.clone(),
        );

        // Create SMTP transport
        let mailer = if self.config.use_tls {
            // Use TLS (port 587 typically)
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
                .map_err(|e| EmailError::ConfigurationError(format!("Failed to create SMTP relay: {}", e)))?
                .credentials(creds)
                .port(self.config.port)
                .build()
        } else {
            // Use plain SMTP (port 25 typically) - only for testing/local development
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host)
                .port(self.config.port)
                .credentials(creds)
                .build()
        };

        // Send the email
        match mailer.send(email).await {
            Ok(_) => {
                info!("📧 [SMTP] Email sent successfully to {}", message.to);
                Ok(())
            }
            Err(e) => {
                error!("📧 [SMTP] Failed to send email to {}: {}", message.to, e);
                Err(EmailError::SendFailed(format!("SMTP send failed: {}", e)))
            }
        }
    }

    async fn verify_address(&self, email: &str) -> Result<bool, EmailError> {
        // In production, you might use an email verification service
        // For now, just do basic validation

        let is_valid = email.contains('@')
            && email.contains('.')
            && email.len() > 5
            && !email.contains(' ');

        Ok(is_valid)
    }
}

/// SendGrid email sender (alternative to SMTP)
pub struct SendGridEmailSender {
    api_key: String,
    from_email: String,
    from_name: String,
}

impl SendGridEmailSender {
    pub fn from_env() -> Result<Self, EmailError> {
        Ok(Self {
            api_key: std::env::var("SENDGRID_API_KEY")
                .map_err(|_| EmailError::ConfigurationError("SENDGRID_API_KEY not set".to_string()))?,
            from_email: std::env::var("SENDGRID_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@quake.app".to_string()),
            from_name: std::env::var("SENDGRID_FROM_NAME")
                .unwrap_or_else(|_| "Quake App".to_string()),
        })
    }
}

#[async_trait]
impl EmailService for SendGridEmailSender {
    async fn send_email(&self, message: EmailMessage) -> Result<(), EmailError> {
        use sendgrid::v3::{Sender, Email, Content, Personalization, Message as SGMessage};

        info!(
            "📧 [SendGrid] Sending email to {} with subject: {}",
            message.to, message.subject
        );

        // Create SendGrid sender (with optional HTTP client)
        let sender = Sender::new(self.api_key.clone(), None);

        // Build the email
        let mut sg_message = SGMessage::new(
            Email::new(&self.from_email).set_name(&self.from_name),
        );

        // Add personalization (recipient)
        let personalization = Personalization::new(Email::new(&message.to));
        sg_message = sg_message.add_personalization(personalization);

        // Add subject
        sg_message = sg_message.set_subject(&message.subject);

        // Add content (both text and HTML)
        sg_message = sg_message.add_content(Content::new()
            .set_content_type("text/plain")
            .set_value(&message.body_text));

        sg_message = sg_message.add_content(Content::new()
            .set_content_type("text/html")
            .set_value(&message.body_html));

        // Send the email
        match sender.send(&sg_message).await {
            Ok(_) => {
                info!("📧 [SendGrid] Email sent successfully to {}", message.to);
                Ok(())
            }
            Err(e) => {
                error!("📧 [SendGrid] Failed to send email to {}: {}", message.to, e);
                Err(EmailError::SendFailed(format!("SendGrid API error: {}", e)))
            }
        }
    }

    async fn verify_address(&self, email: &str) -> Result<bool, EmailError> {
        // Basic validation for now
        let is_valid = email.contains('@')
            && email.contains('.')
            && email.len() > 5
            && !email.contains(' ');

        Ok(is_valid)
    }
}