//! Integration Tests for Email Verification Process - Enterprise Grade
//!
//! This module contains comprehensive integration tests for the complete email verification
//! flow, including multi-provider support, internationalization, accessibility, fraud
//! detection, bulk operations, and advanced enterprise features.

#[cfg(test)]
mod email_verification_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;

    use crate::errors::AppError;
    use crate::models::{User, UserStatus};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration as StdDuration;
    use tokio::time::{sleep, Duration as TokioDuration};
    use serde::{Serialize, Deserialize};
    use std::collections::{HashMap, HashSet};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct EmailVerificationToken {
        token: String,
        user_id: Uuid,
        email: String,
        token_type: TokenType,
        language: String,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        used: bool,
        attempts: u32,
        ip_address: Option<String>,
        user_agent: Option<String>,
        fraud_score: Option<f64>,
        tracking_id: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum TokenType {
        EmailVerification,
        EmailChange,
        PasswordReset,
        TwoFactorSetup,
        AccountRecovery,
    }

    #[derive(Debug, Clone)]
    struct EmailChangeRequest {
        user_id: Uuid,
        old_email: String,
        new_email: String,
        requested_at: chrono::DateTime<Utc>,
        verified_old: bool,
        verified_new: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct EmailLog {
        to: String,
        subject: String,
        body: String,
        sent_at: chrono::DateTime<Utc>,
        email_type: String,
        language: String,
        provider: String,
        message_id: String,
        delivery_status: EmailDeliveryStatus,
        opened: bool,
        clicked: bool,
        bounced: bool,
        bounce_reason: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    enum EmailDeliveryStatus {
        Queued,
        Sent,
        Delivered,
        Failed,
        Bounced,
        Complained,
    }

    #[derive(Debug, Clone)]
    struct VerificationAttempt {
        token: String,
        ip_address: String,
        attempted_at: chrono::DateTime<Utc>,
        success: bool,
        failure_reason: Option<String>,
    }

    // Enterprise email verification integration simulator
    struct EmailVerificationSimulator {
        users: Vec<User>,
        verification_tokens: Vec<EmailVerificationToken>,
        email_change_requests: Vec<EmailChangeRequest>,
        sent_emails: Vec<EmailLog>,
        verification_attempts: Vec<VerificationAttempt>,
        blocked_emails: Vec<String>,
        rate_limits: Vec<(Uuid, chrono::DateTime<Utc>)>,
        // Enterprise features
        email_providers: Vec<EmailProvider>,
        active_provider: usize,
        email_templates: HashMap<String, EmailTemplate>,
        fraud_detector: FraudDetector,
        analytics: AnalyticsCollector,
        bulk_operations: Vec<BulkOperation>,
        compliance_settings: ComplianceSettings,
        accessibility_features: AccessibilityFeatures,
    }

    // Supporting structures for enterprise features
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct EmailProvider {
        name: String,
        provider_type: ProviderType,
        is_active: bool,
        priority: u32,
        failure_count: u32,
        last_failure: Option<chrono::DateTime<Utc>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    enum ProviderType {
        SMTP { host: String, port: u16 },
        SendGrid { api_key: String },
        AWS_SES { region: String },
        Mailgun { domain: String },
        PostMark,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct EmailTemplate {
        language: String,
        subject: String,
        html_body: String,
        text_body: String,
        accessibility_features: bool,
        rtl_support: bool,
    }

    #[derive(Debug, Clone)]
    struct FraudDetector {
        disposable_domains: HashSet<String>,
        risk_patterns: HashMap<String, f64>,
        ip_reputation: HashMap<String, f64>,
    }

    #[derive(Debug, Clone)]
    struct AnalyticsCollector {
        metrics: HashMap<String, u64>,
        events: Vec<AnalyticsEvent>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AnalyticsEvent {
        event_type: String,
        timestamp: chrono::DateTime<Utc>,
        user_id: Option<Uuid>,
        metadata: HashMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct BulkOperation {
        operation_id: Uuid,
        operation_type: String,
        started_at: chrono::DateTime<Utc>,
        completed_at: Option<chrono::DateTime<Utc>>,
        total_items: usize,
        processed_items: usize,
        successful_items: usize,
        failed_items: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct ComplianceSettings {
        gdpr_enabled: bool,
        ccpa_enabled: bool,
        data_retention_days: u32,
        consent_tracking: bool,
        audit_logging: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AccessibilityFeatures {
        screen_reader_support: bool,
        high_contrast_mode: bool,
        language_detection: bool,
        voice_assistance: bool,
        keyboard_navigation: bool,
    }

    impl FraudDetector {
        fn analyze_email_request(&mut self, email: &str, ip_address: &str, user_agent: Option<&str>) -> f64 {
            let mut risk_score = 0.0;

            // Check disposable email domains
            if let Some(domain) = email.split('@').nth(1) {
                if self.disposable_domains.contains(domain) {
                    risk_score += 0.5;
                }
            }

            // Check IP reputation
            if let Some(&ip_risk) = self.ip_reputation.get(ip_address) {
                risk_score += ip_risk;
            }

            // Check user agent patterns
            if let Some(ua) = user_agent {
                if ua.contains("bot") || ua.contains("curl") || ua.contains("wget") {
                    risk_score += 0.6;
                }
            }

            // Cap at 1.0
            risk_score.min(1.0)
        }
    }

    impl AnalyticsCollector {
        fn record_event(&mut self, event_type: &str, user_id: Option<Uuid>, metadata: HashMap<String, String>) {
            self.events.push(AnalyticsEvent {
                event_type: event_type.to_string(),
                timestamp: Utc::now(),
                user_id,
                metadata,
            });

            *self.metrics.entry(event_type.to_string()).or_insert(0) += 1;
        }

        fn get_metrics(&self) -> &HashMap<String, u64> {
            &self.metrics
        }
    }

    impl EmailVerificationSimulator {
        fn new() -> Self {
            let mut email_templates = HashMap::new();

            // English template
            email_templates.insert("en".to_string(), EmailTemplate {
                language: "en".to_string(),
                subject: "Verify your email address".to_string(),
                html_body: r#"<html><head><meta charset="UTF-8"><title>Email Verification</title></head><body><h1>Verify Your Email</h1><p>Click <a href="{verification_url}" aria-label="Verify email address">here</a> to verify.</p></body></html>"#.to_string(),
                text_body: "Verify your email: {verification_url}".to_string(),
                accessibility_features: true,
                rtl_support: false,
            });

            // Spanish template
            email_templates.insert("es".to_string(), EmailTemplate {
                language: "es".to_string(),
                subject: "Verifica tu dirección de correo".to_string(),
                html_body: r#"<html><head><meta charset="UTF-8"><title>Verificación de Email</title></head><body><h1>Verifica Tu Email</h1><p>Haz clic <a href="{verification_url}" aria-label="Verificar dirección de correo">aquí</a> para verificar.</p></body></html>"#.to_string(),
                text_body: "Verifica tu correo: {verification_url}".to_string(),
                accessibility_features: true,
                rtl_support: false,
            });

            // French template
            email_templates.insert("fr".to_string(), EmailTemplate {
                language: "fr".to_string(),
                subject: "Vérifiez votre adresse e-mail".to_string(),
                html_body: r#"<html><head><meta charset="UTF-8"><title>Vérification d'E-mail</title></head><body><h1>Vérifiez Votre E-mail</h1><p>Cliquez <a href="{verification_url}" aria-label="Vérifier l'adresse e-mail">ici</a> pour vérifier.</p></body></html>"#.to_string(),
                text_body: "Vérifiez votre e-mail: {verification_url}".to_string(),
                accessibility_features: true,
                rtl_support: false,
            });

            let mut disposable_domains = HashSet::new();
            disposable_domains.extend([
                "10minutemail.com", "guerrillamail.com", "temp-mail.org",
                "throwaway.email", "mailinator.com", "spam.com", "fake.net"
            ].iter().map(|s| s.to_string()));

            Self {
                users: Vec::new(),
                verification_tokens: Vec::new(),
                email_change_requests: Vec::new(),
                sent_emails: Vec::new(),
                verification_attempts: Vec::new(),
                blocked_emails: vec!["spam.com".to_string(), "fake.net".to_string()],
                rate_limits: Vec::new(),
                // Enterprise features
                email_providers: vec![
                    EmailProvider {
                        name: "Primary SMTP".to_string(),
                        provider_type: ProviderType::SMTP { host: "smtp.example.com".to_string(), port: 587 },
                        is_active: true,
                        priority: 1,
                        failure_count: 0,
                        last_failure: None,
                    },
                    EmailProvider {
                        name: "SendGrid Backup".to_string(),
                        provider_type: ProviderType::SendGrid { api_key: "key123".to_string() },
                        is_active: true,
                        priority: 2,
                        failure_count: 0,
                        last_failure: None,
                    },
                ],
                active_provider: 0,
                email_templates,
                fraud_detector: FraudDetector {
                    disposable_domains,
                    risk_patterns: HashMap::new(),
                    ip_reputation: HashMap::new(),
                },
                analytics: AnalyticsCollector {
                    metrics: HashMap::new(),
                    events: Vec::new(),
                },
                bulk_operations: Vec::new(),
                compliance_settings: ComplianceSettings {
                    gdpr_enabled: true,
                    ccpa_enabled: true,
                    data_retention_days: 365,
                    consent_tracking: true,
                    audit_logging: true,
                },
                accessibility_features: AccessibilityFeatures {
                    screen_reader_support: true,
                    high_contrast_mode: true,
                    language_detection: true,
                    voice_assistance: false,
                    keyboard_navigation: true,
                },
            }
        }

        // Legacy method for backwards compatibility
        fn request_email_verification(&mut self, user_id: &Uuid, ip_address: String) -> Result<String, AppError> {
            self.request_email_verification_with_language(user_id, ip_address, None, None)
        }

        fn create_unverified_user(&mut self, email: String, username: String) -> User {
            let user = User {
                id: Uuid::new_v4(),
                username,
                email: email.clone(),
                password_hash: "hashed_password".to_string(),
                email_verified: false,
                status: UserStatus::PendingVerification,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            };

            self.users.push(user.clone());
            user
        }

        fn validate_email_format(&self, email: &str) -> Result<(), AppError> {
            // Basic email validation
            let email_parts: Vec<&str> = email.split('@').collect();
            if email.is_empty() ||
               !email.contains('@') ||
               email.contains(' ') ||
               email_parts.len() != 2 ||
               email_parts[0].is_empty() ||
               email_parts[1].is_empty() ||
               !email_parts[1].contains('.') {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Invalid email format".to_string(),
                });
            }

            // Check for blocked domains
            let domain = email_parts[1];
            if self.blocked_emails.iter().any(|blocked| domain == blocked) {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email domain is not allowed".to_string(),
                });
            }

            Ok(())
        }

        fn check_rate_limit(&mut self, user_id: &Uuid) -> Result<(), AppError> {
            let now = Utc::now();
            let rate_limit_window = Duration::minutes(5);

            // Check if user has recent request
            if let Some((_, last_request)) = self.rate_limits.iter().find(|(id, _)| id == user_id) {
                if now - *last_request < rate_limit_window {
                    return Err(AppError::Validation {
                        field: "rate_limit".to_string(),
                        message: "Too many verification requests. Please wait before trying again.".to_string(),
                    });
                }
            }

            // Update rate limit
            self.rate_limits.retain(|(id, _)| id != user_id);
            self.rate_limits.push((*user_id, now));

            Ok(())
        }

        fn request_email_verification_with_language(&mut self, user_id: &Uuid, ip_address: String, user_agent: Option<String>, language: Option<String>) -> Result<String, AppError> {
            // Find user and get email
            let user_email = {
                let user = self.users.iter()
                    .find(|u| u.id == *user_id)
                    .ok_or_else(|| AppError::NotFound {
                        resource: "user".to_string(),
                        id: Some(user_id.to_string()),
                    })?;

                // Check if already verified
                if user.email_verified {
                    return Err(AppError::Validation {
                        field: "email".to_string(),
                        message: "Email is already verified".to_string(),
                    });
                }

                user.email.clone()
            };

            // Check rate limit
            self.check_rate_limit(user_id)?;

            // Fraud detection
            let fraud_score = self.fraud_detector.analyze_email_request(&user_email, &ip_address, user_agent.as_deref());
            if fraud_score > 0.8 {
                return Err(AppError::Validation {
                    field: "fraud_detection".to_string(),
                    message: "Email verification request blocked due to security concerns".to_string(),
                });
            }

            // Invalidate existing tokens
            for token in self.verification_tokens.iter_mut() {
                if token.user_id == *user_id && token.token_type == TokenType::EmailVerification && !token.used {
                    token.used = true;
                }
            }

            // Determine language
            let lang = language.unwrap_or_else(|| "en".to_string());

            // Create new verification token
            let token = format!("verify_{}", Uuid::new_v4());
            let tracking_id = format!("track_{}", Uuid::new_v4());
            let verification_token = EmailVerificationToken {
                token: token.clone(),
                user_id: *user_id,
                email: user_email.clone(),
                token_type: TokenType::EmailVerification,
                language: lang.clone(),
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(24),
                used: false,
                attempts: 0,
                ip_address: Some(ip_address.clone()),
                user_agent: user_agent.clone(),
                fraud_score: Some(fraud_score),
                tracking_id: tracking_id.clone(),
            };

            self.verification_tokens.push(verification_token);

            // Send verification email using template
            let template = self.email_templates.get(&lang).unwrap_or_else(|| self.email_templates.get("en").unwrap());
            let verification_url = format!("https://example.com/verify?token={}&lang={}", token, lang);
            let html_body = template.html_body.replace("{verification_url}", &verification_url);
            let text_body = template.text_body.replace("{verification_url}", &verification_url);

            let message_id = format!("msg_{}", Uuid::new_v4());
            let provider = &self.email_providers[self.active_provider].name;

            let email_log = EmailLog {
                to: user_email,
                subject: template.subject.clone(),
                body: html_body,
                sent_at: Utc::now(),
                email_type: "verification".to_string(),
                language: lang,
                provider: provider.clone(),
                message_id: message_id.clone(),
                delivery_status: EmailDeliveryStatus::Sent,
                opened: false,
                clicked: false,
                bounced: false,
                bounce_reason: None,
            };
            self.sent_emails.push(email_log);

            // Record analytics
            self.analytics.record_event("verification_email_sent", Some(*user_id), {
                let mut metadata = HashMap::new();
                metadata.insert("language".to_string(), lang);
                metadata.insert("fraud_score".to_string(), fraud_score.to_string());
                metadata
            });

            Ok(token)
        }

        fn verify_email(&mut self, token: &str, ip_address: String) -> Result<(), AppError> {
            // Find token
            let token_index = self.verification_tokens.iter()
                .position(|t| t.token == token && t.token_type == TokenType::EmailVerification)
                .ok_or_else(|| {
                    // Log failed attempt
                    let attempt = VerificationAttempt {
                        token: token.to_string(),
                        ip_address: ip_address.clone(),
                        attempted_at: Utc::now(),
                        success: false,
                        failure_reason: Some("Token not found".to_string()),
                    };
                    self.verification_attempts.push(attempt);

                    AppError::Validation {
                        field: "token".to_string(),
                        message: "Invalid verification token".to_string(),
                    }
                })?;

            let verification_token = &mut self.verification_tokens[token_index];

            // Check if token is already used
            if verification_token.used {
                let attempt = VerificationAttempt {
                    token: token.to_string(),
                    ip_address,
                    attempted_at: Utc::now(),
                    success: false,
                    failure_reason: Some("Token already used".to_string()),
                };
                self.verification_attempts.push(attempt);

                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Verification token has already been used".to_string(),
                });
            }

            // Check token expiration
            if verification_token.expires_at < Utc::now() {
                let attempt = VerificationAttempt {
                    token: token.to_string(),
                    ip_address,
                    attempted_at: Utc::now(),
                    success: false,
                    failure_reason: Some("Token expired".to_string()),
                };
                self.verification_attempts.push(attempt);

                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Verification token has expired".to_string(),
                });
            }

            // Check max attempts
            verification_token.attempts += 1;
            if verification_token.attempts > 5 {
                verification_token.used = true;

                let attempt = VerificationAttempt {
                    token: token.to_string(),
                    ip_address,
                    attempted_at: Utc::now(),
                    success: false,
                    failure_reason: Some("Max attempts exceeded".to_string()),
                };
                self.verification_attempts.push(attempt);

                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Maximum verification attempts exceeded".to_string(),
                });
            }

            let user_id = verification_token.user_id;

            // Mark token as used
            verification_token.used = true;

            // Update user
            if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
                user.email_verified = true;
                user.status = UserStatus::Active;
                user.updated_at = Utc::now();
            }

            // Log successful attempt
            let attempt = VerificationAttempt {
                token: token.to_string(),
                ip_address,
                attempted_at: Utc::now(),
                success: true,
                failure_reason: None,
            };
            self.verification_attempts.push(attempt);

            // Send welcome email
            if let Some(user) = self.users.iter().find(|u| u.id == user_id) {
                let email_log = EmailLog {
                    to: user.email.clone(),
                    subject: "Welcome! Your email is verified".to_string(),
                    body: "Thank you for verifying your email. You can now use all features.".to_string(),
                    sent_at: Utc::now(),
                    email_type: "welcome".to_string(),
                };
                self.sent_emails.push(email_log);
            }

            Ok(())
        }

        fn request_email_change(&mut self, user_id: &Uuid, new_email: String, _ip_address: String) -> Result<(String, String), AppError> {
            // Find user and get email
            let old_email = {
                let user = self.users.iter()
                    .find(|u| u.id == *user_id)
                    .ok_or_else(|| AppError::NotFound {
                        resource: "user".to_string(),
                        id: Some(user_id.to_string()),
                    })?;
                user.email.clone()
            };

            // Validate new email
            self.validate_email_format(&new_email)?;

            // Check if new email is already in use
            if self.users.iter().any(|u| u.email == new_email && u.id != *user_id) {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email is already in use".to_string(),
                });
            }

            // Check rate limit
            self.check_rate_limit(user_id)?;

            // Create email change request
            let request = EmailChangeRequest {
                user_id: *user_id,
                old_email: old_email.clone(),
                new_email: new_email.clone(),
                requested_at: Utc::now(),
                verified_old: false,
                verified_new: false,
            };
            self.email_change_requests.push(request);

            // Create tokens for both old and new email
            let old_email_token = format!("verify_old_{}", Uuid::new_v4());
            let new_email_token = format!("verify_new_{}", Uuid::new_v4());

            // Token for old email verification
            let old_token = EmailVerificationToken {
                token: old_email_token.clone(),
                user_id: *user_id,
                email: old_email.clone(),
                token_type: TokenType::EmailChange,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(1), // Shorter expiry for email change
                used: false,
                attempts: 0,
            };
            self.verification_tokens.push(old_token);

            // Token for new email verification
            let new_token = EmailVerificationToken {
                token: new_email_token.clone(),
                user_id: *user_id,
                email: new_email.clone(),
                token_type: TokenType::EmailChange,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(1),
                used: false,
                attempts: 0,
            };
            self.verification_tokens.push(new_token);

            // Send verification emails
            let old_email_log = EmailLog {
                to: old_email,
                subject: "Confirm email change".to_string(),
                body: format!("Click to confirm email change: https://example.com/verify-change?token={}", old_email_token),
                sent_at: Utc::now(),
                email_type: "email_change_old".to_string(),
            };
            self.sent_emails.push(old_email_log);

            let new_email_log = EmailLog {
                to: new_email,
                subject: "Verify new email address".to_string(),
                body: format!("Click to verify new email: https://example.com/verify-change?token={}", new_email_token),
                sent_at: Utc::now(),
                email_type: "email_change_new".to_string(),
            };
            self.sent_emails.push(new_email_log);

            Ok((old_email_token, new_email_token))
        }

        fn verify_email_change_token(&mut self, token: &str, _ip_address: String) -> Result<(), AppError> {
            // Find and validate token
            let token_index = self.verification_tokens.iter()
                .position(|t| t.token == token && t.token_type == TokenType::EmailChange && !t.used)
                .ok_or_else(|| AppError::Validation {
                    field: "token".to_string(),
                    message: "Invalid or expired email change token".to_string(),
                })?;

            let verification_token = &self.verification_tokens[token_index];

            // Check expiration
            if verification_token.expires_at < Utc::now() {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Email change token has expired".to_string(),
                });
            }

            let user_id = verification_token.user_id;
            let is_old_email = token.starts_with("verify_old_");

            // Mark token as used
            self.verification_tokens[token_index].used = true;

            // Update email change request
            if let Some(request) = self.email_change_requests.iter_mut()
                .find(|r| r.user_id == user_id && !r.verified_old || !r.verified_new) {

                if is_old_email {
                    request.verified_old = true;
                } else {
                    request.verified_new = true;
                }

                // If both are verified, complete the email change
                if request.verified_old && request.verified_new {
                    let new_email = request.new_email.clone();

                    // Update user email
                    if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
                        user.email = new_email.clone();
                        user.updated_at = Utc::now();

                        // Send confirmation email
                        let email_log = EmailLog {
                            to: new_email,
                            subject: "Email change successful".to_string(),
                            body: "Your email address has been successfully changed.".to_string(),
                            sent_at: Utc::now(),
                            email_type: "email_change_confirmation".to_string(),
                        };
                        self.sent_emails.push(email_log);
                    }
                }
            }

            Ok(())
        }

        fn resend_verification(&mut self, user_id: &Uuid, ip_address: String) -> Result<String, AppError> {
            // Check if user exists and needs verification
            let user = self.users.iter()
                .find(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            if user.email_verified {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email is already verified".to_string(),
                });
            }

            // Check for existing valid token
            if let Some(existing_token) = self.verification_tokens.iter()
                .find(|t| t.user_id == *user_id &&
                      t.token_type == TokenType::EmailVerification &&
                      !t.used &&
                      t.expires_at > Utc::now()) {

                // Resend email with existing token
                let email_log = EmailLog {
                    to: user.email.clone(),
                    subject: "Verify your email address (Resent)".to_string(),
                    body: format!("Click here to verify: https://example.com/verify?token={}", existing_token.token),
                    sent_at: Utc::now(),
                    email_type: "verification_resend".to_string(),
                };
                self.sent_emails.push(email_log);

                return Ok(existing_token.token.clone());
            }

            // Create new token if no valid one exists
            self.request_email_verification(user_id, ip_address)
        }

        fn cleanup_expired_tokens(&mut self) {
            let now = Utc::now();
            for token in self.verification_tokens.iter_mut() {
                if token.expires_at < now && !token.used {
                    token.used = true;
                }
            }
        }

        fn get_verification_status(&self, user_id: &Uuid) -> Result<(bool, Option<chrono::DateTime<Utc>>), AppError> {
            let user = self.users.iter()
                .find(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Get latest token expiry if unverified
            let token_expiry = if !user.email_verified {
                self.verification_tokens.iter()
                    .filter(|t| t.user_id == *user_id &&
                           t.token_type == TokenType::EmailVerification &&
                           !t.used)
                    .map(|t| t.expires_at)
                    .max()
            } else {
                None
            };

            Ok((user.email_verified, token_expiry))
        }

        fn get_verification_attempts(&self, token: &str) -> Vec<&VerificationAttempt> {
            self.verification_attempts.iter()
                .filter(|a| a.token == token)
                .collect()
        }

        fn block_email_domain(&mut self, domain: String) {
            if !self.blocked_emails.contains(&domain) {
                self.blocked_emails.push(domain);
            }
        }

        fn unblock_email_domain(&mut self, domain: &str) {
            self.blocked_emails.retain(|d| d != domain);
        }

        // Enterprise features

        async fn bulk_send_verification_emails(&mut self, user_emails: Vec<(Uuid, String)>, language: Option<String>) -> Result<BulkOperation, AppError> {
            let operation_id = Uuid::new_v4();
            let started_at = Utc::now();
            let mut successful_items = 0;
            let mut failed_items = 0;

            for (user_id, email) in &user_emails {
                match self.request_email_verification_with_language(user_id, "bulk_operation".to_string(), None, language.clone()) {
                    Ok(_) => successful_items += 1,
                    Err(_) => failed_items += 1,
                }
            }

            let bulk_op = BulkOperation {
                operation_id,
                operation_type: "bulk_verification".to_string(),
                started_at,
                completed_at: Some(Utc::now()),
                total_items: user_emails.len(),
                processed_items: user_emails.len(),
                successful_items,
                failed_items,
            };

            self.bulk_operations.push(bulk_op.clone());
            self.analytics.record_event("bulk_verification_completed", None, {
                let mut metadata = HashMap::new();
                metadata.insert("operation_id".to_string(), operation_id.to_string());
                metadata.insert("total_items".to_string(), user_emails.len().to_string());
                metadata.insert("successful_items".to_string(), successful_items.to_string());
                metadata
            });

            Ok(bulk_op)
        }

        fn simulate_email_open(&mut self, message_id: &str) -> Result<(), AppError> {
            if let Some(email) = self.sent_emails.iter_mut().find(|e| e.message_id == message_id) {
                email.opened = true;
                email.delivery_status = EmailDeliveryStatus::Delivered;
            }
            Ok(())
        }

        fn simulate_email_click(&mut self, message_id: &str) -> Result<(), AppError> {
            if let Some(email) = self.sent_emails.iter_mut().find(|e| e.message_id == message_id) {
                email.clicked = true;
                email.opened = true;
                email.delivery_status = EmailDeliveryStatus::Delivered;
            }
            Ok(())
        }

        fn simulate_email_bounce(&mut self, message_id: &str, bounce_reason: String) -> Result<(), AppError> {
            if let Some(email) = self.sent_emails.iter_mut().find(|e| e.message_id == message_id) {
                email.bounced = true;
                email.bounce_reason = Some(bounce_reason);
                email.delivery_status = EmailDeliveryStatus::Bounced;
            }
            Ok(())
        }

        fn failover_to_next_provider(&mut self) -> Result<(), AppError> {
            // Mark current provider as failed
            if let Some(provider) = self.email_providers.get_mut(self.active_provider) {
                provider.failure_count += 1;
                provider.last_failure = Some(Utc::now());
                if provider.failure_count > 3 {
                    provider.is_active = false;
                }
            }

            // Find next active provider
            let mut next_provider = None;
            for (index, provider) in self.email_providers.iter().enumerate() {
                if provider.is_active && index != self.active_provider {
                    next_provider = Some(index);
                    break;
                }
            }

            if let Some(next) = next_provider {
                self.active_provider = next;
                self.analytics.record_event("provider_failover", None, {
                    let mut metadata = HashMap::new();
                    metadata.insert("new_provider".to_string(), next.to_string());
                    metadata
                });
                Ok(())
            } else {
                Err(AppError::Internal {
                    message: "No active email providers available".to_string(),
                })
            }
        }

        fn get_email_analytics(&self) -> HashMap<String, u64> {
            let mut analytics = HashMap::new();
            analytics.insert("total_emails_sent".to_string(), self.sent_emails.len() as u64);
            analytics.insert("emails_opened".to_string(), self.sent_emails.iter().filter(|e| e.opened).count() as u64);
            analytics.insert("emails_clicked".to_string(), self.sent_emails.iter().filter(|e| e.clicked).count() as u64);
            analytics.insert("emails_bounced".to_string(), self.sent_emails.iter().filter(|e| e.bounced).count() as u64);

            let mut by_language = HashMap::new();
            for email in &self.sent_emails {
                *by_language.entry(&email.language).or_insert(0u64) += 1;
            }

            for (lang, count) in by_language {
                analytics.insert(format!("emails_sent_{}", lang), count);
            }

            analytics
        }

        fn gdpr_delete_user_data(&mut self, user_id: &Uuid) -> Result<(), AppError> {
            if !self.compliance_settings.gdpr_enabled {
                return Err(AppError::Validation {
                    field: "compliance".to_string(),
                    message: "GDPR compliance not enabled".to_string(),
                });
            }

            // Remove user data
            self.users.retain(|u| u.id != *user_id);
            self.verification_tokens.retain(|t| t.user_id != *user_id);
            self.email_change_requests.retain(|r| r.user_id != *user_id);
            self.sent_emails.retain(|e| !self.users.iter().any(|u| u.email == e.to));
            self.verification_attempts.retain(|a| !self.verification_tokens.iter().any(|t| t.token == a.token && t.user_id == *user_id));

            self.analytics.record_event("gdpr_deletion", Some(*user_id), HashMap::new());
            Ok(())
        }

        fn test_accessibility_compliance(&self) -> Result<bool, AppError> {
            for template in self.email_templates.values() {
                if !template.accessibility_features {
                    return Ok(false);
                }

                // Check for accessibility features in HTML
                if !template.html_body.contains("aria-label") {
                    return Ok(false);
                }

                // Check for proper encoding
                if !template.html_body.contains("charset=UTF-8") {
                    return Ok(false);
                }
            }
            Ok(true)
        }

        fn simulate_concurrent_verifications(&mut self, user_count: usize, verification_count_per_user: usize) -> Result<Vec<String>, AppError> {
            let mut all_tokens = Vec::new();

            for i in 0..user_count {
                let user = self.create_unverified_user(
                    format!("concurrent_user_{}@example.com", i),
                    format!("concurrent_user_{}", i),
                );

                // Clear rate limits for testing
                self.rate_limits.clear();

                for j in 0..verification_count_per_user {
                    match self.request_email_verification_with_language(
                        &user.id,
                        format!("192.168.1.{}", i + 1),
                        Some(format!("TestAgent/1.0 (concurrent test {})", j)),
                        Some("en".to_string()),
                    ) {
                        Ok(token) => all_tokens.push(token),
                        Err(_) => {} // Some may fail due to rate limiting or fraud detection
                    }
                }
            }

            Ok(all_tokens)
        }
    }

    #[test]
    fn test_basic_email_verification_flow() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Request verification
        let token = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();
        assert!(!token.is_empty());
        assert!(token.starts_with("verify_"));

        // Verify email was sent
        assert_eq!(simulator.sent_emails.len(), 1);
        assert_eq!(simulator.sent_emails[0].to, "user@example.com");
        assert_eq!(simulator.sent_emails[0].email_type, "verification");

        // Verify email using token
        let result = simulator.verify_email(&token, "192.168.1.100".to_string());
        assert!(result.is_ok());

        // Check user is now verified
        let (verified, _) = simulator.get_verification_status(&user.id).unwrap();
        assert!(verified);

        // Check welcome email was sent
        assert_eq!(simulator.sent_emails.len(), 2);
        assert_eq!(simulator.sent_emails[1].email_type, "welcome");

        // Verify user status updated
        let updated_user = simulator.users.iter().find(|u| u.id == user.id).unwrap();
        assert!(updated_user.email_verified);
        assert_eq!(updated_user.status, UserStatus::Active);
    }

    #[test]
    fn test_verification_with_invalid_token() {
        let mut simulator = EmailVerificationSimulator::new();

        let result = simulator.verify_email("invalid_token", "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("Invalid"));
            }
            _ => panic!("Expected validation error"),
        }

        // Check failed attempt was logged
        let attempts = simulator.get_verification_attempts("invalid_token");
        assert_eq!(attempts.len(), 1);
        assert!(!attempts[0].success);
        assert_eq!(attempts[0].failure_reason, Some("Token not found".to_string()));
    }

    #[test]
    fn test_verification_token_expiration() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let token = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Manually expire the token
        if let Some(token_ref) = simulator.verification_tokens.iter_mut().find(|t| t.token == token) {
            token_ref.expires_at = Utc::now() - Duration::hours(1);
        }

        // Try to verify with expired token
        let result = simulator.verify_email(&token, "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_verification_token_reuse_prevention() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let token = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // First verification should succeed
        let result1 = simulator.verify_email(&token, "192.168.1.100".to_string());
        assert!(result1.is_ok());

        // Second verification with same token should fail
        let result2 = simulator.verify_email(&token, "192.168.1.100".to_string());
        assert!(result2.is_err());

        match result2.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("already been used"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_rate_limiting() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // First request should succeed
        let result1 = simulator.request_email_verification(&user.id, "192.168.1.100".to_string());
        assert!(result1.is_ok());

        // Immediate second request should be rate limited
        let result2 = simulator.request_email_verification(&user.id, "192.168.1.100".to_string());
        assert!(result2.is_err());

        match result2.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "rate_limit");
                assert!(message.contains("Too many"));
            }
            _ => panic!("Expected rate limit error"),
        }
    }

    #[test]
    fn test_email_change_flow() {
        let mut simulator = EmailVerificationSimulator::new();

        let mut user = simulator.create_unverified_user(
            "old@example.com".to_string(),
            "testuser".to_string()
        );

        // Verify initial email first
        user.email_verified = true;
        user.status = UserStatus::Active;
        simulator.users[0] = user.clone();

        // Request email change
        let (old_token, new_token) = simulator.request_email_change(
            &user.id,
            "new@example.com".to_string(),
            "192.168.1.100".to_string()
        ).unwrap();

        // Verify emails were sent to both addresses
        let email_to_old = simulator.sent_emails.iter()
            .find(|e| e.to == "old@example.com" && e.email_type == "email_change_old");
        let email_to_new = simulator.sent_emails.iter()
            .find(|e| e.to == "new@example.com" && e.email_type == "email_change_new");

        assert!(email_to_old.is_some());
        assert!(email_to_new.is_some());

        // Verify old email token
        simulator.verify_email_change_token(&old_token, "192.168.1.100".to_string()).unwrap();

        // Email shouldn't be changed yet
        let user_check = simulator.users.iter().find(|u| u.id == user.id).unwrap();
        assert_eq!(user_check.email, "old@example.com");

        // Verify new email token
        simulator.verify_email_change_token(&new_token, "192.168.1.100".to_string()).unwrap();

        // Now email should be changed
        let updated_user = simulator.users.iter().find(|u| u.id == user.id).unwrap();
        assert_eq!(updated_user.email, "new@example.com");

        // Confirmation email should be sent
        let confirmation = simulator.sent_emails.iter()
            .find(|e| e.email_type == "email_change_confirmation");
        assert!(confirmation.is_some());
    }

    #[test]
    fn test_blocked_email_domain() {
        let mut simulator = EmailVerificationSimulator::new();

        // Try to create user with blocked domain
        let user = simulator.create_unverified_user(
            "user@allowed.com".to_string(),
            "testuser".to_string()
        );

        // Try to change to blocked domain
        let result = simulator.request_email_change(
            &user.id,
            "user@spam.com".to_string(),
            "192.168.1.100".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert!(message.contains("not allowed"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_resend_verification() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Request initial verification
        let token1 = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Clear rate limit for testing
        simulator.rate_limits.clear();

        // Resend verification
        let token2 = simulator.resend_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Should get the same token (reusing existing valid token)
        assert_eq!(token1, token2);

        // Check that resend email was sent
        let resend_emails: Vec<_> = simulator.sent_emails.iter()
            .filter(|e| e.email_type == "verification_resend")
            .collect();
        assert_eq!(resend_emails.len(), 1);
    }

    #[test]
    fn test_max_verification_attempts() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let token = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Manually set attempts to near max
        if let Some(token_ref) = simulator.verification_tokens.iter_mut().find(|t| t.token == token) {
            token_ref.attempts = 4;
        }

        // This should succeed (5th attempt)
        let result1 = simulator.verify_email(&token, "192.168.1.100".to_string());
        assert!(result1.is_ok());

        // Token should now be used
        let token_status = simulator.verification_tokens.iter()
            .find(|t| t.token == token)
            .unwrap();
        assert!(token_status.used);
    }

    #[test]
    fn test_email_validation() {
        let simulator = EmailVerificationSimulator::new();

        let invalid_emails = vec![
            "",
            "notanemail",
            "@example.com",
            "user@",
            "user @example.com",
            "user@spam.com", // blocked domain
        ];

        for invalid_email in invalid_emails {
            let result = simulator.validate_email_format(invalid_email);
            assert!(result.is_err());
        }

        let valid_emails = vec![
            "user@example.com",
            "test.user+tag@subdomain.example.org",
            "user123@test.co.uk",
        ];

        for valid_email in valid_emails {
            let result = simulator.validate_email_format(valid_email);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_duplicate_email_prevention() {
        let mut simulator = EmailVerificationSimulator::new();

        let _user1 = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "user1".to_string()
        );

        let user2 = simulator.create_unverified_user(
            "other@example.com".to_string(),
            "user2".to_string()
        );

        // Try to change user2's email to user1's email
        let result = simulator.request_email_change(
            &user2.id,
            "user@example.com".to_string(),
            "192.168.1.100".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert!(message.contains("already in use"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_token_cleanup() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Create multiple tokens
        let token1 = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Clear rate limit for testing
        simulator.rate_limits.clear();

        let token2 = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Manually expire first token
        if let Some(token_ref) = simulator.verification_tokens.iter_mut().find(|t| t.token == token1) {
            token_ref.expires_at = Utc::now() - Duration::hours(1);
        }

        // Run cleanup
        simulator.cleanup_expired_tokens();

        // First token should be marked as used
        let token1_status = simulator.verification_tokens.iter()
            .find(|t| t.token == token1)
            .unwrap();
        assert!(token1_status.used);

        // Second token should still be valid
        let token2_status = simulator.verification_tokens.iter()
            .find(|t| t.token == token2)
            .unwrap();
        assert!(!token2_status.used);
    }

    #[test]
    fn test_verification_already_verified() {
        let mut simulator = EmailVerificationSimulator::new();

        let mut user = simulator.create_unverified_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Mark as already verified
        user.email_verified = true;
        user.status = UserStatus::Active;
        simulator.users[0] = user.clone();

        // Try to request verification
        let result = simulator.request_email_verification(&user.id, "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert!(message.contains("already verified"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_complete_verification_workflow() {
        let mut simulator = EmailVerificationSimulator::new();

        // Step 1: Create unverified user
        let user = simulator.create_unverified_user(
            "newuser@example.com".to_string(),
            "newuser".to_string()
        );
        assert!(!user.email_verified);
        assert_eq!(user.status, UserStatus::PendingVerification);

        // Step 2: Request verification
        let token = simulator.request_email_verification(&user.id, "192.168.1.100".to_string()).unwrap();

        // Step 3: Check verification status
        let (verified, token_expiry) = simulator.get_verification_status(&user.id).unwrap();
        assert!(!verified);
        assert!(token_expiry.is_some());

        // Step 4: Verify email
        simulator.verify_email(&token, "192.168.1.100".to_string()).unwrap();

        // Step 5: Check final status
        let (verified, token_expiry) = simulator.get_verification_status(&user.id).unwrap();
        assert!(verified);
        assert!(token_expiry.is_none());

        // Verify all emails sent
        assert_eq!(simulator.sent_emails.len(), 2); // verification + welcome

        // Verify successful attempt logged
        let attempts = simulator.get_verification_attempts(&token);
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].success);
    }

    // Enterprise Integration Tests

    #[test]
    fn test_multi_language_verification() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "multiuser@example.com".to_string(),
            "multiuser".to_string(),
        );

        // Test different languages
        let languages = vec!["en", "es", "fr"];
        for lang in languages {
            // Clear rate limits for testing
            simulator.rate_limits.clear();

            let token = simulator.request_email_verification_with_language(
                &user.id,
                "127.0.0.1".to_string(),
                Some("Mozilla/5.0".to_string()),
                Some(lang.to_string()),
            ).unwrap();

            // Check that email was sent with correct language
            let sent_email = simulator.sent_emails.iter()
                .find(|e| e.language == lang)
                .unwrap();

            assert_eq!(sent_email.language, lang);
            assert!(sent_email.body.contains(&token));

            // Verify token has language set
            let verification_token = simulator.verification_tokens.iter()
                .find(|t| t.token == token)
                .unwrap();
            assert_eq!(verification_token.language, lang);
        }
    }

    #[test]
    fn test_fraud_detection() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "suspicious@guerrillamail.com".to_string(),
            "suspicious_user".to_string(),
        );

        // Request with suspicious characteristics
        let result = simulator.request_email_verification_with_language(
            &user.id,
            "127.0.0.1".to_string(),
            Some("curl/7.68.0".to_string()), // Bot user agent
            Some("en".to_string()),
        );

        // Should be blocked due to high fraud score
        assert!(result.is_err());
        if let Err(AppError::Validation { field, message }) = result {
            assert_eq!(field, "fraud_detection");
            assert!(message.contains("security concerns"));
        }
    }

    #[tokio::test]
    async fn test_bulk_verification_operations() {
        let mut simulator = EmailVerificationSimulator::new();

        let user_emails = vec![
            (Uuid::new_v4(), "bulk1@example.com".to_string()),
            (Uuid::new_v4(), "bulk2@example.com".to_string()),
            (Uuid::new_v4(), "bulk3@example.com".to_string()),
        ];

        // Create users first
        for (user_id, email) in &user_emails {
            simulator.users.push(User {
                id: *user_id,
                username: email.split('@').next().unwrap().to_string(),
                email: email.clone(),
                password_hash: "hashed".to_string(),
                email_verified: false,
                status: UserStatus::PendingVerification,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            });
        }

        let bulk_result = simulator.bulk_send_verification_emails(user_emails.clone(), Some("en".to_string())).await.unwrap();

        assert_eq!(bulk_result.total_items, 3);
        assert_eq!(bulk_result.successful_items, 3);
        assert_eq!(bulk_result.failed_items, 0);
        assert!(bulk_result.completed_at.is_some());

        // Check that emails were sent
        assert!(simulator.sent_emails.len() >= 3);

        // Check analytics
        let metrics = simulator.analytics.get_metrics();
        assert!(metrics.get("bulk_verification_completed").unwrap_or(&0) > &0);
    }

    #[test]
    fn test_email_delivery_tracking() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "tracked@example.com".to_string(),
            "tracked_user".to_string(),
        );

        let token = simulator.request_email_verification(&user.id, "127.0.0.1".to_string()).unwrap();

        // Find the sent email
        let sent_email = simulator.sent_emails.iter()
            .find(|e| e.to == "tracked@example.com")
            .unwrap();
        let message_id = sent_email.message_id.clone();

        // Simulate email tracking events
        simulator.simulate_email_open(&message_id).unwrap();
        simulator.simulate_email_click(&message_id).unwrap();

        // Verify tracking was recorded
        let updated_email = simulator.sent_emails.iter()
            .find(|e| e.message_id == message_id)
            .unwrap();

        assert!(updated_email.opened);
        assert!(updated_email.clicked);
        assert_eq!(updated_email.delivery_status, EmailDeliveryStatus::Delivered);
    }

    #[test]
    fn test_email_bounce_handling() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "bounce@example.com".to_string(),
            "bounce_user".to_string(),
        );

        let token = simulator.request_email_verification(&user.id, "127.0.0.1".to_string()).unwrap();

        // Find the sent email
        let sent_email = simulator.sent_emails.iter()
            .find(|e| e.to == "bounce@example.com")
            .unwrap();
        let message_id = sent_email.message_id.clone();

        // Simulate bounce
        simulator.simulate_email_bounce(&message_id, "Mailbox full".to_string()).unwrap();

        // Verify bounce was recorded
        let bounced_email = simulator.sent_emails.iter()
            .find(|e| e.message_id == message_id)
            .unwrap();

        assert!(bounced_email.bounced);
        assert_eq!(bounced_email.bounce_reason, Some("Mailbox full".to_string()));
        assert_eq!(bounced_email.delivery_status, EmailDeliveryStatus::Bounced);
    }

    #[test]
    fn test_provider_failover() {
        let mut simulator = EmailVerificationSimulator::new();

        // Initially using provider 0
        assert_eq!(simulator.active_provider, 0);

        // Simulate provider failure
        simulator.failover_to_next_provider().unwrap();

        // Should now be using provider 1
        assert_eq!(simulator.active_provider, 1);

        // Check that failure was recorded
        assert_eq!(simulator.email_providers[0].failure_count, 1);
        assert!(simulator.email_providers[0].last_failure.is_some());

        // Check analytics
        let metrics = simulator.analytics.get_metrics();
        assert!(metrics.get("provider_failover").unwrap_or(&0) > &0);
    }

    #[test]
    fn test_email_analytics() {
        let mut simulator = EmailVerificationSimulator::new();

        let user1 = simulator.create_unverified_user("analytics1@example.com".to_string(), "user1".to_string());
        let user2 = simulator.create_unverified_user("analytics2@example.com".to_string(), "user2".to_string());

        // Send emails in different languages
        simulator.request_email_verification_with_language(&user1.id, "127.0.0.1".to_string(), None, Some("en".to_string())).unwrap();
        simulator.rate_limits.clear(); // Clear for second user
        simulator.request_email_verification_with_language(&user2.id, "127.0.0.1".to_string(), None, Some("es".to_string())).unwrap();

        let analytics = simulator.get_email_analytics();

        assert_eq!(analytics.get("total_emails_sent").unwrap(), &2);
        assert_eq!(analytics.get("emails_sent_en").unwrap(), &1);
        assert_eq!(analytics.get("emails_sent_es").unwrap(), &1);
        assert_eq!(analytics.get("emails_opened").unwrap(), &0);
        assert_eq!(analytics.get("emails_clicked").unwrap(), &0);
        assert_eq!(analytics.get("emails_bounced").unwrap(), &0);
    }

    #[test]
    fn test_gdpr_compliance() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "gdpr@example.com".to_string(),
            "gdpr_user".to_string(),
        );

        let user_id = user.id;

        // Send verification email
        simulator.request_email_verification(&user_id, "127.0.0.1".to_string()).unwrap();

        // Verify data exists
        assert!(simulator.users.iter().any(|u| u.id == user_id));
        assert!(simulator.verification_tokens.iter().any(|t| t.user_id == user_id));
        assert!(simulator.sent_emails.iter().any(|e| e.to == "gdpr@example.com"));

        // Perform GDPR deletion
        simulator.gdpr_delete_user_data(&user_id).unwrap();

        // Verify data is removed
        assert!(!simulator.users.iter().any(|u| u.id == user_id));
        assert!(!simulator.verification_tokens.iter().any(|t| t.user_id == user_id));

        // Check analytics
        let metrics = simulator.analytics.get_metrics();
        assert!(metrics.get("gdpr_deletion").unwrap_or(&0) > &0);
    }

    #[test]
    fn test_accessibility_compliance() {
        let simulator = EmailVerificationSimulator::new();

        let is_compliant = simulator.test_accessibility_compliance().unwrap();
        assert!(is_compliant);

        // Check individual template features
        for template in simulator.email_templates.values() {
            assert!(template.accessibility_features);
            assert!(template.html_body.contains("aria-label"));
            assert!(template.html_body.contains("charset=UTF-8"));
        }
    }

    #[test]
    fn test_concurrent_verification_simulation() {
        let mut simulator = EmailVerificationSimulator::new();

        let tokens = simulator.simulate_concurrent_verifications(5, 2).unwrap();

        // Should have generated tokens (some may have failed due to fraud detection)
        assert!(!tokens.is_empty());

        // Check that users were created
        let concurrent_users = simulator.users.iter()
            .filter(|u| u.username.starts_with("concurrent_user_"))
            .count();
        assert_eq!(concurrent_users, 5);

        // Check that emails were sent
        let concurrent_emails = simulator.sent_emails.iter()
            .filter(|e| e.to.starts_with("concurrent_user_"))
            .count();
        assert!(concurrent_emails > 0);
    }

    #[test]
    fn test_enterprise_token_features() {
        let mut simulator = EmailVerificationSimulator::new();

        let user = simulator.create_unverified_user(
            "enterprise@example.com".to_string(),
            "enterprise_user".to_string(),
        );

        let token = simulator.request_email_verification_with_language(
            &user.id,
            "192.168.1.100".to_string(),
            Some("Mozilla/5.0 (Enterprise)".to_string()),
            Some("en".to_string()),
        ).unwrap();

        let verification_token = simulator.verification_tokens.iter()
            .find(|t| t.token == token)
            .unwrap();

        // Check enterprise features
        assert_eq!(verification_token.language, "en");
        assert_eq!(verification_token.ip_address, Some("192.168.1.100".to_string()));
        assert_eq!(verification_token.user_agent, Some("Mozilla/5.0 (Enterprise)".to_string()));
        assert!(verification_token.fraud_score.is_some());
        assert!(!verification_token.tracking_id.is_empty());
    }

    #[test]
    fn test_email_template_localization() {
        let simulator = EmailVerificationSimulator::new();

        // Test all supported languages
        for lang in &["en", "es", "fr"] {
            let template = simulator.email_templates.get(*lang).unwrap();
            assert_eq!(template.language, *lang);
            assert!(!template.subject.is_empty());
            assert!(!template.html_body.is_empty());
            assert!(!template.text_body.is_empty());
            assert!(template.html_body.contains("{verification_url}"));
            assert!(template.text_body.contains("{verification_url}"));
        }
    }

    #[test]
    fn test_comprehensive_verification_workflow() {
        let mut simulator = EmailVerificationSimulator::new();

        // Step 1: Create user
        let user = simulator.create_unverified_user(
            "workflow@example.com".to_string(),
            "workflow_user".to_string(),
        );

        // Step 2: Request verification with full context
        let token = simulator.request_email_verification_with_language(
            &user.id,
            "192.168.1.100".to_string(),
            Some("Mozilla/5.0 (Workflow Test)".to_string()),
            Some("en".to_string()),
        ).unwrap();

        // Step 3: Simulate email delivery tracking
        let sent_email = simulator.sent_emails.iter()
            .find(|e| e.to == "workflow@example.com")
            .unwrap();
        let message_id = sent_email.message_id.clone();

        simulator.simulate_email_open(&message_id).unwrap();

        // Step 4: Verify email
        simulator.verify_email(&token, "192.168.1.100".to_string()).unwrap();

        // Step 5: Validate final state
        let updated_user = simulator.users.iter().find(|u| u.id == user.id).unwrap();
        assert!(updated_user.email_verified);
        assert_eq!(updated_user.status, UserStatus::Active);

        let verification_token = simulator.verification_tokens.iter()
            .find(|t| t.token == token)
            .unwrap();
        assert!(verification_token.used);

        let analytics = simulator.get_email_analytics();
        assert!(analytics.get("total_emails_sent").unwrap() > &0);
        assert!(analytics.get("emails_opened").unwrap() > &0);

        let core_metrics = simulator.analytics.get_metrics();
        assert!(core_metrics.get("verification_email_sent").unwrap_or(&0) > &0);
    }
}