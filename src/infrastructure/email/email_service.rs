use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Email message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub to: String,
    pub from: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

/// Email error types
#[derive(Debug)]
pub enum EmailError {
    SendFailed(String),
    InvalidAddress(String),
    ConfigurationError(String),
    RateLimitExceeded,
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmailError::SendFailed(msg) => write!(f, "Failed to send email: {}", msg),
            EmailError::InvalidAddress(addr) => write!(f, "Invalid email address: {}", addr),
            EmailError::ConfigurationError(msg) => write!(f, "Email configuration error: {}", msg),
            EmailError::RateLimitExceeded => write!(f, "Email rate limit exceeded"),
        }
    }
}

impl std::error::Error for EmailError {}

/// Email service trait - allows different implementations
#[async_trait]
pub trait EmailService: Send + Sync {
    /// Send a single email
    async fn send_email(&self, message: EmailMessage) -> Result<(), EmailError>;

    /// Send multiple emails (batch)
    async fn send_batch(&self, messages: Vec<EmailMessage>) -> Result<Vec<Result<(), EmailError>>, EmailError> {
        let mut results = Vec::new();
        for message in messages {
            results.push(self.send_email(message).await);
        }
        Ok(results)
    }

    /// Verify if an email address is valid (optional implementation)
    async fn verify_address(&self, email: &str) -> Result<bool, EmailError> {
        // Default implementation - just check format
        Ok(email.contains('@') && email.contains('.'))
    }
}

/// Email template builder for common emails
pub struct EmailTemplates;

impl EmailTemplates {
    /// Create verification email
    pub fn verification_email(
        to: &str,
        from: &str,
        username: &str,
        verification_url: &str,
    ) -> EmailMessage {
        let subject = "Verify your email address".to_string();

        let body_html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .button {{
                        display: inline-block;
                        padding: 12px 24px;
                        background-color: #007bff;
                        color: white;
                        text-decoration: none;
                        border-radius: 4px;
                        margin: 20px 0;
                    }}
                    .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; font-size: 12px; color: #666; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h2>Welcome, {}!</h2>
                    <p>Thank you for registering. Please verify your email address to activate your account.</p>
                    <p>Click the button below to verify your email:</p>
                    <a href="{}" class="button">Verify Email</a>
                    <p>Or copy and paste this link into your browser:</p>
                    <p style="word-break: break-all; color: #007bff;">{}</p>
                    <p>This link will expire in 24 hours.</p>
                    <div class="footer">
                        <p>If you didn't create an account, you can safely ignore this email.</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            username, verification_url, verification_url
        );

        let body_text = format!(
            "Welcome, {}!\n\n\
            Thank you for registering. Please verify your email address to activate your account.\n\n\
            Click this link to verify your email:\n{}\n\n\
            This link will expire in 24 hours.\n\n\
            If you didn't create an account, you can safely ignore this email.",
            username, verification_url
        );

        EmailMessage {
            to: to.to_string(),
            from: from.to_string(),
            subject,
            body_html,
            body_text,
        }
    }

    /// Create password reset email
    pub fn password_reset_email(
        to: &str,
        from: &str,
        username: &str,
        reset_url: &str,
    ) -> EmailMessage {
        let subject = "Reset your password".to_string();

        let body_html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .button {{
                        display: inline-block;
                        padding: 12px 24px;
                        background-color: #dc3545;
                        color: white;
                        text-decoration: none;
                        border-radius: 4px;
                        margin: 20px 0;
                    }}
                    .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; font-size: 12px; color: #666; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h2>Password Reset Request</h2>
                    <p>Hi {},</p>
                    <p>We received a request to reset your password. Click the button below to create a new password:</p>
                    <a href="{}" class="button">Reset Password</a>
                    <p>Or copy and paste this link into your browser:</p>
                    <p style="word-break: break-all; color: #dc3545;">{}</p>
                    <p>This link will expire in 1 hour for security reasons.</p>
                    <div class="footer">
                        <p>If you didn't request a password reset, please ignore this email. Your password won't be changed.</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            username, reset_url, reset_url
        );

        let body_text = format!(
            "Password Reset Request\n\n\
            Hi {},\n\n\
            We received a request to reset your password. Click this link to create a new password:\n{}\n\n\
            This link will expire in 1 hour for security reasons.\n\n\
            If you didn't request a password reset, please ignore this email. Your password won't be changed.",
            username, reset_url
        );

        EmailMessage {
            to: to.to_string(),
            from: from.to_string(),
            subject,
            body_html,
            body_text,
        }
    }

    /// Create welcome email (after verification)
    pub fn welcome_email(
        to: &str,
        from: &str,
        username: &str,
    ) -> EmailMessage {
        let subject = "Welcome to Quake!".to_string();

        let body_html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
                    .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
                    .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; font-size: 12px; color: #666; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h2>Welcome to Quake, {}!</h2>
                    <p>Your account has been successfully verified and activated.</p>
                    <p>You can now log in and start using all features of our platform.</p>
                    <h3>Getting Started:</h3>
                    <ul>
                        <li>Complete your profile</li>
                        <li>Explore available features</li>
                        <li>Check out our documentation</li>
                    </ul>
                    <div class="footer">
                        <p>If you have any questions, feel free to contact our support team.</p>
                    </div>
                </div>
            </body>
            </html>
            "#,
            username
        );

        let body_text = format!(
            "Welcome to Quake, {}!\n\n\
            Your account has been successfully verified and activated.\n\n\
            You can now log in and start using all features of our platform.\n\n\
            Getting Started:\n\
            - Complete your profile\n\
            - Explore available features\n\
            - Check out our documentation\n\n\
            If you have any questions, feel free to contact our support team.",
            username
        );

        EmailMessage {
            to: to.to_string(),
            from: from.to_string(),
            subject,
            body_html,
            body_text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_message_creation() {
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<p>Test HTML</p>".to_string(),
            body_text: "Test Text".to_string(),
        };

        assert_eq!(message.to, "test@example.com");
        assert_eq!(message.from, "sender@example.com");
        assert_eq!(message.subject, "Test Subject");
        assert_eq!(message.body_html, "<p>Test HTML</p>");
        assert_eq!(message.body_text, "Test Text");
    }

    #[test]
    fn test_email_message_serialization() {
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<p>Test HTML</p>".to_string(),
            body_text: "Test Text".to_string(),
        };

        let json = serde_json::to_string(&message).expect("Failed to serialize EmailMessage");
        let deserialized: EmailMessage = serde_json::from_str(&json).expect("Failed to deserialize EmailMessage");

        assert_eq!(deserialized.to, message.to);
        assert_eq!(deserialized.from, message.from);
        assert_eq!(deserialized.subject, message.subject);
        assert_eq!(deserialized.body_html, message.body_html);
        assert_eq!(deserialized.body_text, message.body_text);
    }

    #[test]
    fn test_email_message_clone() {
        let message = EmailMessage {
            to: "test@example.com".to_string(),
            from: "sender@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body_html: "<p>Test HTML</p>".to_string(),
            body_text: "Test Text".to_string(),
        };

        let cloned = message.clone();
        assert_eq!(cloned.to, message.to);
        assert_eq!(cloned.from, message.from);
        assert_eq!(cloned.subject, message.subject);
        assert_eq!(cloned.body_html, message.body_html);
        assert_eq!(cloned.body_text, message.body_text);
    }

    #[test]
    fn test_email_error_display() {
        let send_failed = EmailError::SendFailed("SMTP error".to_string());
        assert_eq!(send_failed.to_string(), "Failed to send email: SMTP error");

        let invalid_address = EmailError::InvalidAddress("invalid-email".to_string());
        assert_eq!(invalid_address.to_string(), "Invalid email address: invalid-email");

        let config_error = EmailError::ConfigurationError("Missing API key".to_string());
        assert_eq!(config_error.to_string(), "Email configuration error: Missing API key");

        let rate_limit = EmailError::RateLimitExceeded;
        assert_eq!(rate_limit.to_string(), "Email rate limit exceeded");
    }

    #[test]
    fn test_email_error_debug() {
        let error = EmailError::SendFailed("Connection timeout".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("SendFailed"));
        assert!(debug_str.contains("Connection timeout"));
    }

    #[test]
    fn test_verification_email_template() {
        let to = "user@example.com";
        let from = "noreply@quake.app";
        let username = "TestUser";
        let verification_url = "https://example.com/verify?token=abc123";

        let email = EmailTemplates::verification_email(to, from, username, verification_url);

        assert_eq!(email.to, to);
        assert_eq!(email.from, from);
        assert_eq!(email.subject, "Verify your email address");

        // Check HTML content
        assert!(email.body_html.contains(username));
        assert!(email.body_html.contains(verification_url));
        assert!(email.body_html.contains("Welcome"));
        assert!(email.body_html.contains("Verify Email"));
        assert!(email.body_html.contains("24 hours"));

        // Check text content
        assert!(email.body_text.contains(username));
        assert!(email.body_text.contains(verification_url));
        assert!(email.body_text.contains("Welcome"));
        assert!(email.body_text.contains("24 hours"));
    }

    #[test]
    fn test_password_reset_email_template() {
        let to = "user@example.com";
        let from = "noreply@quake.app";
        let username = "TestUser";
        let reset_url = "https://example.com/reset?token=def456";

        let email = EmailTemplates::password_reset_email(to, from, username, reset_url);

        assert_eq!(email.to, to);
        assert_eq!(email.from, from);
        assert_eq!(email.subject, "Reset your password");

        // Check HTML content
        assert!(email.body_html.contains(username));
        assert!(email.body_html.contains(reset_url));
        assert!(email.body_html.contains("Password Reset Request"));
        assert!(email.body_html.contains("Reset Password"));
        assert!(email.body_html.contains("1 hour"));

        // Check text content
        assert!(email.body_text.contains(username));
        assert!(email.body_text.contains(reset_url));
        assert!(email.body_text.contains("Password Reset Request"));
        assert!(email.body_text.contains("1 hour"));
    }

    #[test]
    fn test_welcome_email_template() {
        let to = "user@example.com";
        let from = "noreply@quake.app";
        let username = "TestUser";

        let email = EmailTemplates::welcome_email(to, from, username);

        assert_eq!(email.to, to);
        assert_eq!(email.from, from);
        assert_eq!(email.subject, "Welcome to Quake!");

        // Check HTML content
        assert!(email.body_html.contains(username));
        assert!(email.body_html.contains("Welcome to Quake"));
        assert!(email.body_html.contains("successfully verified"));
        assert!(email.body_html.contains("Getting Started"));
        assert!(email.body_html.contains("Complete your profile"));

        // Check text content
        assert!(email.body_text.contains(username));
        assert!(email.body_text.contains("Welcome to Quake"));
        assert!(email.body_text.contains("successfully verified"));
        assert!(email.body_text.contains("Getting Started"));
        assert!(email.body_text.contains("Complete your profile"));
    }

    #[test]
    fn test_verification_email_escaping() {
        let username = "User<script>";
        let verification_url = "https://example.com/verify?token=abc&param=value";

        let email = EmailTemplates::verification_email("test@example.com", "from@example.com", username, verification_url);

        // Username should be present (basic HTML escaping would be in practice)
        assert!(email.body_html.contains(username));
        assert!(email.body_text.contains(username));

        // URL should be present and properly handled
        assert!(email.body_html.contains(verification_url));
        assert!(email.body_text.contains(verification_url));
    }

    #[test]
    fn test_password_reset_email_escaping() {
        let username = "User&quot;test";
        let reset_url = "https://example.com/reset?token=abc&param=value";

        let email = EmailTemplates::password_reset_email("test@example.com", "from@example.com", username, reset_url);

        assert!(email.body_html.contains(username));
        assert!(email.body_text.contains(username));
        assert!(email.body_html.contains(reset_url));
        assert!(email.body_text.contains(reset_url));
    }

    #[test]
    fn test_welcome_email_escaping() {
        let username = "User<>&\"'";

        let email = EmailTemplates::welcome_email("test@example.com", "from@example.com", username);

        assert!(email.body_html.contains(username));
        assert!(email.body_text.contains(username));
    }

    #[test]
    fn test_email_template_empty_fields() {
        let email = EmailTemplates::verification_email("", "", "", "");

        assert_eq!(email.to, "");
        assert_eq!(email.from, "");
        assert!(email.body_html.contains("Welcome, !"));
        assert!(email.body_text.contains("Welcome, !"));
    }

    #[test]
    fn test_email_template_unicode() {
        let username = "用户测试";
        let to = "用户@example.com";
        let from = "نظام@example.com";
        let url = "https://example.com/verify?user=用户";

        let email = EmailTemplates::verification_email(to, from, username, url);

        assert_eq!(email.to, to);
        assert_eq!(email.from, from);
        assert!(email.body_html.contains(username));
        assert!(email.body_text.contains(username));
        assert!(email.body_html.contains(url));
        assert!(email.body_text.contains(url));
    }

    #[test]
    fn test_email_template_long_content() {
        let long_username = "A".repeat(1000);
        let long_url = format!("https://example.com/verify?token={}", "x".repeat(500));

        let email = EmailTemplates::verification_email("test@example.com", "from@example.com", &long_username, &long_url);

        assert!(email.body_html.contains(&long_username));
        assert!(email.body_text.contains(&long_username));
        assert!(email.body_html.contains(&long_url));
        assert!(email.body_text.contains(&long_url));
    }

    #[test]
    fn test_email_template_special_characters() {
        let special_username = "User!@#$%^&*()_+-=[]{}|;':\",./<>?`~";
        let special_url = "https://example.com/verify?token=abc!@#$%^&*()";

        let email = EmailTemplates::verification_email("test@example.com", "from@example.com", special_username, special_url);

        assert!(email.body_html.contains(special_username));
        assert!(email.body_text.contains(special_username));
        assert!(email.body_html.contains(special_url));
        assert!(email.body_text.contains(special_url));
    }

    #[test]
    fn test_email_html_structure() {
        let email = EmailTemplates::verification_email("test@example.com", "from@example.com", "User", "https://example.com");

        // Check HTML structure
        assert!(email.body_html.contains("<!DOCTYPE html>"));
        assert!(email.body_html.contains("<html>"));
        assert!(email.body_html.contains("<head>"));
        assert!(email.body_html.contains("<body>"));
        assert!(email.body_html.contains("<style>"));
        assert!(email.body_html.contains("class=\"container\""));
        assert!(email.body_html.contains("class=\"button\""));
        assert!(email.body_html.contains("class=\"footer\""));
    }

    #[test]
    fn test_email_styling() {
        let email = EmailTemplates::verification_email("test@example.com", "from@example.com", "User", "https://example.com");

        // Check for key styling elements
        assert!(email.body_html.contains("font-family"));
        assert!(email.body_html.contains("background-color"));
        assert!(email.body_html.contains("border-radius"));
        assert!(email.body_html.contains("padding"));
        assert!(email.body_html.contains("margin"));
    }

    #[test]
    fn test_password_reset_email_styling() {
        let email = EmailTemplates::password_reset_email("test@example.com", "from@example.com", "User", "https://example.com");

        // Check for different button color in reset email
        assert!(email.body_html.contains("#dc3545")); // Red color for reset button
        assert!(email.body_html.contains("Reset Password"));
    }

    #[test]
    fn test_welcome_email_no_button() {
        let email = EmailTemplates::welcome_email("test@example.com", "from@example.com", "User");

        // Welcome email should not have buttons
        assert!(!email.body_html.contains("class=\"button\""));
        assert!(email.body_html.contains("<ul>"));
        assert!(email.body_html.contains("<li>"));
    }

    #[test]
    fn test_all_email_templates_have_footer() {
        let verification = EmailTemplates::verification_email("test@example.com", "from@example.com", "User", "https://example.com");
        let reset = EmailTemplates::password_reset_email("test@example.com", "from@example.com", "User", "https://example.com");
        let welcome = EmailTemplates::welcome_email("test@example.com", "from@example.com", "User");

        assert!(verification.body_html.contains("class=\"footer\""));
        assert!(reset.body_html.contains("class=\"footer\""));
        assert!(welcome.body_html.contains("class=\"footer\""));
    }

    // Test error trait implementation
    #[test]
    fn test_email_error_as_std_error() {
        let error = EmailError::SendFailed("Test error".to_string());
        let std_error: &dyn std::error::Error = &error;
        assert!(std_error.to_string().contains("Test error"));
    }
}