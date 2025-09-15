//! Integration Tests for Email Verification Flow with Token Management
//!
//! This module contains integration tests for the complete email verification process.
//! These tests simulate the full verification flow without requiring a live database.

#[cfg(test)]
mod email_verification_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    #[derive(Debug, Clone)]
    struct EmailVerificationToken {
        token: String,
        user_id: Uuid,
        email: String,
        token_type: TokenType,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        used: bool,
        attempts: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum TokenType {
        EmailVerification,
        EmailChange,
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

    #[derive(Debug, Clone)]
    struct EmailLog {
        to: String,
        subject: String,
        body: String,
        sent_at: chrono::DateTime<Utc>,
        email_type: String,
    }

    #[derive(Debug, Clone)]
    struct VerificationAttempt {
        token: String,
        ip_address: String,
        attempted_at: chrono::DateTime<Utc>,
        success: bool,
        failure_reason: Option<String>,
    }

    // Mock email verification simulator
    struct EmailVerificationSimulator {
        users: Vec<User>,
        verification_tokens: Vec<EmailVerificationToken>,
        email_change_requests: Vec<EmailChangeRequest>,
        sent_emails: Vec<EmailLog>,
        verification_attempts: Vec<VerificationAttempt>,
        blocked_emails: Vec<String>, // Blacklisted email domains
        rate_limits: Vec<(Uuid, chrono::DateTime<Utc>)>, // user_id, last_request_time
    }

    impl EmailVerificationSimulator {
        fn new() -> Self {
            Self {
                users: Vec::new(),
                verification_tokens: Vec::new(),
                email_change_requests: Vec::new(),
                sent_emails: Vec::new(),
                verification_attempts: Vec::new(),
                blocked_emails: vec!["spam.com".to_string(), "fake.net".to_string()],
                rate_limits: Vec::new(),
            }
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

        fn request_email_verification(&mut self, user_id: &Uuid, _ip_address: String) -> Result<String, AppError> {
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

            // Invalidate existing tokens
            for token in self.verification_tokens.iter_mut() {
                if token.user_id == *user_id && token.token_type == TokenType::EmailVerification && !token.used {
                    token.used = true;
                }
            }

            // Create new verification token
            let token = format!("verify_{}", Uuid::new_v4());
            let verification_token = EmailVerificationToken {
                token: token.clone(),
                user_id: *user_id,
                email: user_email.clone(),
                token_type: TokenType::EmailVerification,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(24),
                used: false,
                attempts: 0,
            };

            self.verification_tokens.push(verification_token);

            // Send verification email
            let email_log = EmailLog {
                to: user_email,
                subject: "Verify your email address".to_string(),
                body: format!("Click here to verify: https://example.com/verify?token={}", token),
                sent_at: Utc::now(),
                email_type: "verification".to_string(),
            };
            self.sent_emails.push(email_log);

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
}