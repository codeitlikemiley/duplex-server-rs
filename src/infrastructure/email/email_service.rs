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