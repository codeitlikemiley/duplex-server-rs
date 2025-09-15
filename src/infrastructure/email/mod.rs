pub mod email_service;
pub mod console_email_sender;
pub mod smtp_email_sender;
pub mod email_factory;

pub use email_service::{EmailService, EmailMessage, EmailError, EmailTemplates};
pub use console_email_sender::ConsoleEmailSender;
pub use smtp_email_sender::{SmtpEmailSender, SmtpConfig};
pub use email_factory::{EmailServiceFactory, EmailServiceType};