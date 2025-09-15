//! Tests for Email Verification Service
//!
//! This module contains comprehensive tests for email verification,
//! including token generation, validation, expiration, and email sending.

#[cfg(test)]
mod email_verification_service_tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use std::collections::HashMap;

    use crate::application::services::EmailVerificationService;
    use crate::errors::AppError;

    // Mock verification token structure
    #[derive(Debug, Clone)]
    struct TestVerificationToken {
        id: Uuid,
        user_id: Uuid,
        email: String,
        token: String,
        created_at: chrono::DateTime<chrono::Utc>,
        expires_at: chrono::DateTime<chrono::Utc>,
        is_used: bool,
        attempts: i32,
    }

    // Mock email service for testing
    struct MockEmailService {
        sent_emails: Vec<SentEmail>,
        should_fail: bool,
    }

    #[derive(Debug, Clone)]
    struct SentEmail {
        to: String,
        subject: String,
        body: String,
        sent_at: chrono::DateTime<chrono::Utc>,
    }

    impl MockEmailService {
        fn new() -> Self {
            Self {
                sent_emails: vec![],
                should_fail: false,
            }
        }

        fn send_email(&mut self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
            if self.should_fail {
                return Err(AppError::Internal {
                    message: "Failed to send email".to_string(),
                });
            }

            self.sent_emails.push(SentEmail {
                to: to.to_string(),
                subject: subject.to_string(),
                body: body.to_string(),
                sent_at: Utc::now(),
            });

            Ok(())
        }

        fn get_sent_emails(&self) -> &Vec<SentEmail> {
            &self.sent_emails
        }

        fn clear_sent_emails(&mut self) {
            self.sent_emails.clear();
        }
    }

    // Mock email verification service for testing
    struct TestEmailVerificationService {
        tokens: HashMap<String, TestVerificationToken>,
        email_service: MockEmailService,
        token_expiry_duration: Duration,
        max_attempts: i32,
    }

    impl TestEmailVerificationService {
        fn new() -> Self {
            Self {
                tokens: HashMap::new(),
                email_service: MockEmailService::new(),
                token_expiry_duration: Duration::hours(24),
                max_attempts: 3,
            }
        }

        fn send_verification_email(&mut self, user_id: Uuid, email: &str) -> Result<String, AppError> {
            // Invalidate any existing tokens for this user
            self.invalidate_user_tokens(user_id);

            let token = self.generate_token();
            let now = Utc::now();

            let verification_token = TestVerificationToken {
                id: Uuid::new_v4(),
                user_id,
                email: email.to_string(),
                token: token.clone(),
                created_at: now,
                expires_at: now + self.token_expiry_duration,
                is_used: false,
                attempts: 0,
            };

            // Send email
            let subject = "Verify your email address";
            let body = format!(
                "Please click the following link to verify your email: \
                https://example.com/verify?token={}",
                token
            );

            self.email_service.send_email(email, subject, &body)?;
            self.tokens.insert(token.clone(), verification_token);

            Ok(token)
        }

        fn verify_email(&mut self, token: &str) -> Result<(Uuid, String), AppError> {
            // First, check if token exists and get basic info
            let (user_id, email, is_used, expires_at, attempts) = {
                let verification_token = self.tokens.get(token)
                    .ok_or_else(|| AppError::NotFound {
                        resource: "VerificationToken".to_string(),
                        id: Some(token.to_string()),
                    })?;

                (
                    verification_token.user_id,
                    verification_token.email.clone(),
                    verification_token.is_used,
                    verification_token.expires_at,
                    verification_token.attempts,
                )
            };

            if is_used {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Verification token has already been used".to_string(),
                });
            }

            if Utc::now() > expires_at {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Verification token has expired".to_string(),
                });
            }

            if attempts >= self.max_attempts {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Maximum verification attempts exceeded".to_string(),
                });
            }

            // Now update the token
            if let Some(verification_token) = self.tokens.get_mut(token) {
                verification_token.is_used = true;
                verification_token.attempts += 1;
            }

            Ok((user_id, email))
        }

        fn resend_verification_email(&mut self, user_id: Uuid, email: &str) -> Result<String, AppError> {
            // Check if there's a recent token (within 5 minutes)
            let recent_token = self.tokens.values()
                .find(|t| t.user_id == user_id &&
                      t.email == email &&
                      (Utc::now() - t.created_at).num_minutes() < 5);

            if recent_token.is_some() {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Please wait before requesting another verification email".to_string(),
                });
            }

            self.send_verification_email(user_id, email)
        }

        fn cleanup_expired_tokens(&mut self) -> Result<i32, AppError> {
            let expired_tokens: Vec<String> = self.tokens.iter()
                .filter(|(_, token)| self.is_token_expired(token))
                .map(|(key, _)| key.clone())
                .collect();

            let count = expired_tokens.len() as i32;
            for token in expired_tokens {
                self.tokens.remove(&token);
            }

            Ok(count)
        }

        fn get_verification_status(&self, user_id: Uuid) -> Result<Vec<TestVerificationToken>, AppError> {
            let user_tokens: Vec<TestVerificationToken> = self.tokens.values()
                .filter(|t| t.user_id == user_id)
                .cloned()
                .collect();

            Ok(user_tokens)
        }

        fn invalidate_token(&mut self, token: &str) -> Result<(), AppError> {
            let verification_token = self.tokens.get_mut(token)
                .ok_or_else(|| AppError::NotFound {
                    resource: "VerificationToken".to_string(),
                    id: Some(token.to_string()),
                })?;

            verification_token.is_used = true;
            Ok(())
        }

        // Helper methods
        fn generate_token(&self) -> String {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..32)
                .map(|_| {
                    let idx = rng.gen_range(0..62);
                    match idx {
                        0..=25 => (b'a' + idx) as char,
                        26..=51 => (b'A' + (idx - 26)) as char,
                        _ => (b'0' + (idx - 52)) as char,
                    }
                })
                .collect()
        }

        fn is_token_expired(&self, token: &TestVerificationToken) -> bool {
            Utc::now() > token.expires_at
        }

        fn invalidate_user_tokens(&mut self, user_id: Uuid) {
            for token in self.tokens.values_mut() {
                if token.user_id == user_id {
                    token.is_used = true;
                }
            }
        }

        fn set_email_failure(&mut self, should_fail: bool) {
            self.email_service.should_fail = should_fail;
        }

        fn get_sent_emails(&self) -> &Vec<SentEmail> {
            self.email_service.get_sent_emails()
        }
    }

    #[test]
    fn test_send_verification_email() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();

        // Token should not be empty
        assert!(!token.is_empty());

        // Email should have been sent
        let sent_emails = service.get_sent_emails();
        assert_eq!(sent_emails.len(), 1);
        assert_eq!(sent_emails[0].to, email);
        assert!(sent_emails[0].subject.contains("Verify"));
        assert!(sent_emails[0].body.contains(&token));
    }

    #[test]
    fn test_verify_email_success() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();
        let (verified_user_id, verified_email) = service.verify_email(&token).unwrap();

        assert_eq!(verified_user_id, user_id);
        assert_eq!(verified_email, email);
    }

    #[test]
    fn test_verify_email_invalid_token() {
        let mut service = TestEmailVerificationService::new();
        let invalid_token = "invalid-token";

        let result = service.verify_email(invalid_token);
        assert!(result.is_err());

        if let Err(AppError::NotFound { resource, .. }) = result {
            assert_eq!(resource, "VerificationToken");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_verify_email_already_used() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();

        // First verification should succeed
        assert!(service.verify_email(&token).is_ok());

        // Second verification should fail
        let result = service.verify_email(&token);
        assert!(result.is_err());

        if let Err(AppError::Validation { message, .. }) = result {
            assert!(message.contains("already been used"));
        } else {
            panic!("Expected Validation error for used token");
        }
    }

    #[test]
    fn test_verify_email_expired_token() {
        let mut service = TestEmailVerificationService::new();
        service.token_expiry_duration = Duration::seconds(1);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();

        // Wait for token to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        let result = service.verify_email(&token);
        assert!(result.is_err());

        if let Err(AppError::Validation { message, .. }) = result {
            assert!(message.contains("expired"));
        } else {
            panic!("Expected Validation error for expired token");
        }
    }

    #[test]
    fn test_resend_verification_email() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        // Send initial email
        let token1 = service.send_verification_email(user_id, email).unwrap();

        // Wait a bit (but not enough for resend cooldown)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Resend should fail due to cooldown
        let result = service.resend_verification_email(user_id, email);
        assert!(result.is_err());

        if let Err(AppError::Validation { message, .. }) = result {
            assert!(message.contains("wait"));
        } else {
            panic!("Expected Validation error for resend cooldown");
        }

        // Manually adjust token creation time to simulate cooldown period passed
        for token_data in service.tokens.values_mut() {
            if token_data.user_id == user_id {
                token_data.created_at = Utc::now() - Duration::minutes(10);
            }
        }

        // Now resend should work
        let token2 = service.resend_verification_email(user_id, email).unwrap();
        assert_ne!(token1, token2);

        // Should have 2 emails sent
        let sent_emails = service.get_sent_emails();
        assert_eq!(sent_emails.len(), 2);
    }

    #[test]
    fn test_cleanup_expired_tokens() {
        let mut service = TestEmailVerificationService::new();
        service.token_expiry_duration = Duration::seconds(1);

        let user_id = Uuid::new_v4();

        // Create tokens that will expire
        service.send_verification_email(user_id, "test1@example.com").unwrap();
        service.send_verification_email(user_id, "test2@example.com").unwrap();

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Create a fresh token
        service.token_expiry_duration = Duration::hours(1);
        service.send_verification_email(user_id, "test3@example.com").unwrap();

        // Cleanup should remove 2 expired tokens
        let cleaned_count = service.cleanup_expired_tokens().unwrap();
        assert_eq!(cleaned_count, 2);

        // Fresh token should still exist
        let status = service.get_verification_status(user_id).unwrap();
        let active_tokens: Vec<_> = status.iter()
            .filter(|t| !t.is_used && !service.is_token_expired(t))
            .collect();
        assert_eq!(active_tokens.len(), 1);
        assert_eq!(active_tokens[0].email, "test3@example.com");
    }

    #[test]
    fn test_get_verification_status() {
        let mut service = TestEmailVerificationService::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // Create tokens for different users
        service.send_verification_email(user1, "user1@example.com").unwrap();
        service.send_verification_email(user1, "user1alt@example.com").unwrap();
        service.send_verification_email(user2, "user2@example.com").unwrap();

        let user1_status = service.get_verification_status(user1).unwrap();
        let user2_status = service.get_verification_status(user2).unwrap();

        assert_eq!(user1_status.len(), 2);
        assert_eq!(user2_status.len(), 1);

        // Verify user ownership
        for token in &user1_status {
            assert_eq!(token.user_id, user1);
        }
        for token in &user2_status {
            assert_eq!(token.user_id, user2);
        }
    }

    #[test]
    fn test_invalidate_token() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();

        // Token should be valid initially
        assert!(service.verify_email(&token).is_ok());

        // Create a new token and invalidate it
        let token2 = service.send_verification_email(user_id, "test2@example.com").unwrap();
        service.invalidate_token(&token2).unwrap();

        // Invalidated token should not be verifiable
        let result = service.verify_email(&token2);
        assert!(result.is_err());
    }

    #[test]
    fn test_email_sending_failure() {
        let mut service = TestEmailVerificationService::new();
        service.set_email_failure(true);

        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let result = service.send_verification_email(user_id, email);
        assert!(result.is_err());

        if let Err(AppError::Internal { message }) = result {
            assert!(message.contains("Failed to send email"));
        } else {
            panic!("Expected Internal error for email failure");
        }

        // No emails should have been sent
        assert_eq!(service.get_sent_emails().len(), 0);
    }

    #[test]
    fn test_token_uniqueness() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let mut tokens = std::collections::HashSet::new();

        // Generate multiple tokens
        for i in 0..10 {
            let token = service.send_verification_email(
                user_id,
                &format!("test{}@example.com", i)
            ).unwrap();

            assert!(!tokens.contains(&token), "Token should be unique");
            tokens.insert(token);
        }

        assert_eq!(tokens.len(), 10);
    }

    #[test]
    fn test_multiple_verification_attempts() {
        let mut service = TestEmailVerificationService::new();
        service.max_attempts = 2;

        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();

        // First attempt should succeed
        assert!(service.verify_email(&token).is_ok());

        // Reset token for testing multiple failed attempts
        let token2 = service.send_verification_email(user_id, "test2@example.com").unwrap();

        // Simulate failed attempts by manually incrementing
        if let Some(verification_token) = service.tokens.get_mut(&token2) {
            verification_token.attempts = 2;
            verification_token.is_used = false; // Reset for testing
        }

        let result = service.verify_email(&token2);
        assert!(result.is_err());

        if let Err(AppError::Validation { message, .. }) = result {
            assert!(message.contains("Maximum verification attempts"));
        } else {
            panic!("Expected Validation error for max attempts");
        }
    }

    #[test]
    fn test_concurrent_email_verification() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let service = Arc::new(Mutex::new(TestEmailVerificationService::new()));
        let mut handles = vec![];

        // Send verification emails concurrently with different users
        for i in 0..5 {
            let service_clone = service.clone();
            let handle = thread::spawn(move || {
                let mut service = service_clone.lock().unwrap();
                let user_id = Uuid::new_v4(); // Different user for each thread
                service.send_verification_email(
                    user_id,
                    &format!("test{}@example.com", i)
                )
            });
            handles.push(handle);
        }

        let mut tokens = vec![];
        for handle in handles {
            let token = handle.join().unwrap().unwrap();
            tokens.push(token);
        }

        // All tokens should be unique
        let unique_tokens: std::collections::HashSet<_> = tokens.iter().collect();
        assert_eq!(tokens.len(), unique_tokens.len());

        // Verify all tokens can be used
        let mut service = service.lock().unwrap();
        for token in &tokens {
            assert!(service.verify_email(token).is_ok());
        }
    }

    #[test]
    fn test_email_content_validation() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service.send_verification_email(user_id, email).unwrap();
        let sent_emails = service.get_sent_emails();

        assert_eq!(sent_emails.len(), 1);
        let sent_email = &sent_emails[0];

        // Verify email content
        assert_eq!(sent_email.to, email);
        assert!(sent_email.subject.contains("Verify"));
        assert!(sent_email.body.contains(&token));
        assert!(sent_email.body.contains("https://"));
        assert!(sent_email.sent_at <= Utc::now());
    }

    #[test]
    fn test_edge_cases() {
        let mut service = TestEmailVerificationService::new();
        let user_id = Uuid::new_v4();

        // Test with various email formats
        let email_formats = vec![
            "simple@example.com",
            "user+tag@example.com",
            "user.name@example.co.uk",
            "test123@test-domain.org",
            "用户@example.com", // Unicode
        ];

        for email in email_formats {
            let result = service.send_verification_email(user_id, email);
            assert!(result.is_ok(), "Should handle email format: {}", email);

            let sent_emails = service.get_sent_emails();
            let last_email = sent_emails.last().unwrap();
            assert_eq!(last_email.to, email);
        }
    }
}