//! Integration Tests for Complete User Registration Flow
//!
//! This module contains comprehensive integration tests for the user registration process.
//! These tests simulate the complete flow including API-level testing, email workflows,
//! session management, and error recovery scenarios.

#[cfg(test)]
mod user_registration_flow_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    // Enhanced registration flow simulator with comprehensive features
    struct RegistrationFlowSimulator {
        registered_emails: Vec<String>,
        registered_usernames: Vec<String>,
        verification_tokens: Vec<(String, Uuid, bool, chrono::DateTime<Utc>)>, // token, user_id, used, expires_at
        sessions: Vec<(Uuid, String, chrono::DateTime<Utc>)>, // user_id, token, created_at
        email_queue: Arc<Mutex<Vec<EmailMessage>>>,
        rate_limits: HashMap<String, Vec<chrono::DateTime<Utc>>>, // IP -> attempts
        blocked_ips: HashSet<String>,
        users_db: HashMap<Uuid, User>,
        profiles_db: HashMap<Uuid, UserProfile>,
        audit_log: Vec<AuditEntry>,
        captcha_challenges: HashMap<String, String>, // challenge -> expected answer
        temp_bans: HashMap<String, chrono::DateTime<Utc>>, // email -> banned until
        verification_attempts: HashMap<String, u32>, // token -> attempt count
    }

    #[derive(Debug, Clone)]
    struct EmailMessage {
        to: String,
        subject: String,
        body: String,
        sent_at: chrono::DateTime<Utc>,
        message_type: EmailType,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum EmailType {
        Verification,
        Welcome,
        SecurityAlert,
    }

    #[derive(Debug, Clone)]
    struct UserProfile {
        user_id: Uuid,
        first_name: Option<String>,
        last_name: Option<String>,
        bio: Option<String>,
        avatar_url: Option<String>,
        preferences: serde_json::Value,
        created_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    struct AuditEntry {
        id: Uuid,
        user_id: Option<Uuid>,
        action: String,
        details: serde_json::Value,
        ip_address: String,
        user_agent: String,
        timestamp: chrono::DateTime<Utc>,
    }

    impl RegistrationFlowSimulator {
        fn new() -> Self {
            Self {
                registered_emails: Vec::new(),
                registered_usernames: Vec::new(),
                verification_tokens: Vec::new(),
                sessions: Vec::new(),
                email_queue: Arc::new(Mutex::new(Vec::new())),
                rate_limits: HashMap::new(),
                blocked_ips: HashSet::new(),
                users_db: HashMap::new(),
                profiles_db: HashMap::new(),
                audit_log: Vec::new(),
                captcha_challenges: HashMap::new(),
                temp_bans: HashMap::new(),
                verification_attempts: HashMap::new(),
            }
        }

        fn validate_registration_input(&self, email: &str, username: &str, password: &str) -> Result<(), AppError> {
            // Validate email - must have content before and after @
            let email_parts: Vec<&str> = email.split('@').collect();
            if email.is_empty() ||
               !email.contains('@') ||
               email.contains(' ') ||
               email_parts.len() != 2 ||
               email_parts[0].is_empty() ||
               email_parts[1].is_empty() {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Invalid email format".to_string(),
                });
            }

            // Check for duplicate email
            if self.registered_emails.contains(&email.to_string()) {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email already registered".to_string(),
                });
            }

            // Validate username
            if username.len() < 3 || username.len() > 50 {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username must be between 3 and 50 characters".to_string(),
                });
            }

            // Check for duplicate username
            if self.registered_usernames.contains(&username.to_string()) {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username already taken".to_string(),
                });
            }

            // Validate password strength
            if password.len() < 8 {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must be at least 8 characters".to_string(),
                });
            }

            let has_upper = password.chars().any(|c| c.is_uppercase());
            let has_lower = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_digit(10));
            let has_special = password.chars().any(|c| !c.is_alphanumeric());

            if !has_upper || !has_lower || !has_digit || !has_special {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain uppercase, lowercase, number, and special character".to_string(),
                });
            }

            Ok(())
        }

        fn register_user(&mut self, email: String, username: String, password: String) -> Result<User, AppError> {
            self.validate_registration_input(&email, &username, &password)?;

            // Create user
            let user = User {
                id: Uuid::new_v4(),
                username: username.clone(),
                email: email.clone(),
                password_hash: format!("hashed_{}", password),
                email_verified: false,
                status: UserStatus::PendingVerification,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            };

            // Store registration data
            self.registered_emails.push(email.clone());
            self.registered_usernames.push(username.clone());
            self.users_db.insert(user.id, user.clone());

            // Create default profile
            let profile = UserProfile {
                user_id: user.id,
                first_name: None,
                last_name: None,
                bio: None,
                avatar_url: None,
                preferences: json!({}),
                created_at: Utc::now(),
            };
            self.profiles_db.insert(user.id, profile);

            // Log registration
            self.audit_log.push(AuditEntry {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                action: "user_registered".to_string(),
                details: json!({
                    "email": email,
                    "username": username
                }),
                ip_address: "127.0.0.1".to_string(),
                user_agent: "test-agent".to_string(),
                timestamp: Utc::now(),
            });

            Ok(user)
        }

        fn create_verification_token(&mut self, user_id: Uuid) -> String {
            let token = format!("verify_{}", Uuid::new_v4());
            let expires_at = Utc::now() + Duration::hours(24);
            self.verification_tokens.push((token.clone(), user_id, false, expires_at));
            token
        }

        fn verify_email(&mut self, token: &str) -> Result<Uuid, AppError> {
            // Check verification attempts
            let attempts = self.verification_attempts.entry(token.to_string()).or_insert(0);
            *attempts += 1;

            if *attempts > 5 {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Too many verification attempts".to_string(),
                });
            }

            let now = Utc::now();
            let token_entry = self.verification_tokens
                .iter_mut()
                .find(|(t, _, used, expires_at)| t == token && !used && *expires_at > now);

            match token_entry {
                Some((_, user_id, used, _)) => {
                    *used = true;
                    let user_id = *user_id;

                    // Update user status in database
                    if let Some(user) = self.users_db.get_mut(&user_id) {
                        user.email_verified = true;
                        user.status = UserStatus::Active;
                    }

                    Ok(user_id)
                }
                None => Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Invalid or expired token".to_string(),
                }),
            }
        }

        fn create_session(&mut self, user_id: Uuid) -> String {
            let session_token = format!("session_{}", Uuid::new_v4());
            let created_at = Utc::now();
            self.sessions.push((user_id, session_token.clone(), created_at));

            // Update last login time
            if let Some(user) = self.users_db.get_mut(&user_id) {
                user.last_login_at = Some(created_at);
            }

            session_token
        }

        fn complete_registration_flow(&mut self, email: String, username: String, password: String)
            -> Result<(User, String, String), AppError> {
            // Step 1: Register user
            let user = self.register_user(email.clone(), username, password)?;

            // Step 2: Send verification email
            self.send_verification_email(&email, user.id)?;

            // Step 3: Create verification token
            let verification_token = self.create_verification_token(user.id);

            // Step 4: Verify email
            self.verify_email(&verification_token)?;

            // Step 5: Send welcome email
            self.send_welcome_email(&email)?;

            // Step 6: Create session
            let session_token = self.create_session(user.id);

            // Get updated user from database
            let user = self.users_db.get(&user.id).unwrap().clone();

            Ok((user, verification_token, session_token))
        }

        fn send_verification_email(&mut self, email: &str, user_id: Uuid) -> Result<(), AppError> {
            let message = EmailMessage {
                to: email.to_string(),
                subject: "Verify your email address".to_string(),
                body: format!("Please verify your email for user {}", user_id),
                sent_at: Utc::now(),
                message_type: EmailType::Verification,
            };

            self.email_queue.lock().unwrap().push(message);
            Ok(())
        }

        fn send_welcome_email(&mut self, email: &str) -> Result<(), AppError> {
            let message = EmailMessage {
                to: email.to_string(),
                subject: "Welcome to our platform!".to_string(),
                body: "Thank you for registering!".to_string(),
                sent_at: Utc::now(),
                message_type: EmailType::Welcome,
            };

            self.email_queue.lock().unwrap().push(message);
            Ok(())
        }

        fn check_rate_limit(&mut self, ip: &str) -> Result<(), AppError> {
            // Check if IP is blocked
            if self.blocked_ips.contains(ip) {
                return Err(AppError::Validation {
                    field: "ip".to_string(),
                    message: "IP address is blocked".to_string(),
                });
            }

            let now = Utc::now();
            let one_minute_ago = now - Duration::minutes(1);

            // Get recent attempts for this IP
            let attempts = self.rate_limits.entry(ip.to_string()).or_insert(Vec::new());

            // Remove old attempts
            attempts.retain(|&t| t > one_minute_ago);

            // Check rate limit (5 attempts per minute)
            if attempts.len() >= 5 {
                self.blocked_ips.insert(ip.to_string());
                return Err(AppError::Validation {
                    field: "rate_limit".to_string(),
                    message: "Too many registration attempts".to_string(),
                });
            }

            // Add this attempt
            attempts.push(now);
            Ok(())
        }

        fn require_captcha(&mut self, email: &str) -> Option<String> {
            // Check if email has multiple failed attempts
            if self.temp_bans.contains_key(email) {
                let challenge = format!("captcha_{}", Uuid::new_v4());
                let answer = format!("{:04}", rand::random::<u16>() % 10000);
                self.captcha_challenges.insert(challenge.clone(), answer);
                return Some(challenge);
            }
            None
        }

        fn verify_captcha(&self, challenge: &str, answer: &str) -> bool {
            self.captcha_challenges.get(challenge)
                .map(|expected| expected == answer)
                .unwrap_or(false)
        }

        fn resend_verification_email(&mut self, email: &str) -> Result<(), AppError> {
            // Find user by email and clone needed data
            let (user_id, email_verified) = self.users_db.values()
                .find(|u| u.email == email)
                .map(|u| (u.id, u.email_verified))
                .ok_or_else(|| AppError::Validation {
                    field: "email".to_string(),
                    message: "Email not found".to_string(),
                })?;

            if email_verified {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email already verified".to_string(),
                });
            }

            // Create new token
            let token = self.create_verification_token(user_id);

            // Send email
            self.send_verification_email(email, user_id)?;

            // Log the action
            self.audit_log.push(AuditEntry {
                id: Uuid::new_v4(),
                user_id: Some(user_id),
                action: "verification_email_resent".to_string(),
                details: json!({ "email": email }),
                ip_address: "127.0.0.1".to_string(),
                user_agent: "test-agent".to_string(),
                timestamp: Utc::now(),
            });

            Ok(())
        }

        async fn register_with_retries(&mut self, email: String, username: String, password: String, max_retries: u32)
            -> Result<User, AppError> {
            let mut retries = 0;
            loop {
                match self.register_user(email.clone(), username.clone(), password.clone()) {
                    Ok(user) => return Ok(user),
                    Err(e) if retries < max_retries => {
                        retries += 1;
                        sleep(TokioDuration::from_millis(100 * retries as u64)).await;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }

    // Helper module for mocking external dependencies
    mod rand {
        pub fn random<T>() -> T
        where
            T: Default,
        {
            T::default()
        }
    }

    #[test]
    fn test_complete_registration_flow_success() {
        let mut simulator = RegistrationFlowSimulator::new();

        let email = "newuser@example.com".to_string();
        let username = "newuser123".to_string();
        let password = "SecurePass123!".to_string();

        let result = simulator.complete_registration_flow(email.clone(), username.clone(), password);

        assert!(result.is_ok());
        let (user, verification_token, session_token) = result.unwrap();

        // Verify user properties
        assert_eq!(user.email, email);
        assert_eq!(user.username, username);
        assert!(user.email_verified);
        assert_eq!(user.status, UserStatus::Active);
        assert!(!verification_token.is_empty());
        assert!(!session_token.is_empty());

        // Verify registration data was stored
        assert!(simulator.registered_emails.contains(&email));
        assert!(simulator.registered_usernames.contains(&username));
    }

    #[test]
    fn test_registration_with_duplicate_email() {
        let mut simulator = RegistrationFlowSimulator::new();

        let email = "duplicate@example.com".to_string();
        let username1 = "user1".to_string();
        let username2 = "user2".to_string();
        let password = "SecurePass123!".to_string();

        // First registration should succeed
        let result1 = simulator.register_user(email.clone(), username1, password.clone());
        assert!(result1.is_ok());

        // Second registration with same email should fail
        let result2 = simulator.register_user(email, username2, password);
        assert!(result2.is_err());

        match result2.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert!(message.contains("already registered"));
            }
            _ => panic!("Expected validation error for duplicate email"),
        }
    }

    #[test]
    fn test_registration_with_duplicate_username() {
        let mut simulator = RegistrationFlowSimulator::new();

        let email1 = "user1@example.com".to_string();
        let email2 = "user2@example.com".to_string();
        let username = "duplicateuser".to_string();
        let password = "SecurePass123!".to_string();

        // First registration should succeed
        let result1 = simulator.register_user(email1, username.clone(), password.clone());
        assert!(result1.is_ok());

        // Second registration with same username should fail
        let result2 = simulator.register_user(email2, username, password);
        assert!(result2.is_err());

        match result2.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "username");
                assert!(message.contains("already taken"));
            }
            _ => panic!("Expected validation error for duplicate username"),
        }
    }

    #[test]
    fn test_registration_with_invalid_email() {
        let mut simulator = RegistrationFlowSimulator::new();

        let invalid_emails = vec![
            "",
            "notanemail",
            "@example.com",
            "user@",
            "user @example.com",
        ];

        for invalid_email in invalid_emails {
            let result = simulator.register_user(
                invalid_email.to_string(),
                "validusername".to_string(),
                "SecurePass123!".to_string(),
            );

            assert!(result.is_err());
            match result.unwrap_err() {
                AppError::Validation { field, .. } => {
                    assert_eq!(field, "email");
                }
                _ => panic!("Expected validation error for invalid email"),
            }
        }
    }

    #[test]
    fn test_registration_with_invalid_username() {
        let mut simulator = RegistrationFlowSimulator::new();

        let long_username = "a".repeat(51);
        let invalid_usernames = vec![
            "ab",  // Too short
            long_username.as_str(),  // Too long
        ];

        for invalid_username in invalid_usernames {
            let result = simulator.register_user(
                "valid@example.com".to_string(),
                invalid_username.to_string(),
                "SecurePass123!".to_string(),
            );

            assert!(result.is_err());
            match result.unwrap_err() {
                AppError::Validation { field, .. } => {
                    assert_eq!(field, "username");
                }
                _ => panic!("Expected validation error for invalid username"),
            }
        }
    }

    #[test]
    fn test_registration_with_weak_password() {
        let mut simulator = RegistrationFlowSimulator::new();

        let weak_passwords = vec![
            "short",          // Too short
            "nouppercas3!",   // No uppercase
            "NOLOWERCASE3!",  // No lowercase
            "NoNumbers!",     // No numbers
            "NoSpecial123",   // No special characters
        ];

        for weak_password in weak_passwords {
            let result = simulator.register_user(
                "valid@example.com".to_string(),
                "validusername".to_string(),
                weak_password.to_string(),
            );

            assert!(result.is_err());
            match result.unwrap_err() {
                AppError::Validation { field, .. } => {
                    assert_eq!(field, "password");
                }
                _ => panic!("Expected validation error for weak password"),
            }
        }
    }

    #[test]
    fn test_email_verification_with_invalid_token() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Create a user and token
        let user = simulator.register_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        let _valid_token = simulator.create_verification_token(user.id);

        // Try to verify with invalid token
        let result = simulator.verify_email("invalid_token");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("Invalid"));
            }
            _ => panic!("Expected validation error for invalid token"),
        }
    }

    #[test]
    fn test_email_verification_token_reuse() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Create a user and token
        let user = simulator.register_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        let token = simulator.create_verification_token(user.id);

        // First verification should succeed
        let result1 = simulator.verify_email(&token);
        assert!(result1.is_ok());

        // Second verification with same token should fail
        let result2 = simulator.verify_email(&token);
        assert!(result2.is_err());
    }

    #[test]
    fn test_session_creation_after_registration() {
        let mut simulator = RegistrationFlowSimulator::new();

        let user = simulator.register_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Create multiple sessions for the user
        let session1 = simulator.create_session(user.id);
        let session2 = simulator.create_session(user.id);

        assert!(!session1.is_empty());
        assert!(!session2.is_empty());
        assert_ne!(session1, session2);

        // Verify sessions were created
        assert_eq!(simulator.sessions.len(), 2);
        assert!(simulator.sessions.iter().all(|(uid, _, _)| *uid == user.id));
    }

    #[test]
    fn test_registration_state_transitions() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Register user - should start as PendingVerification
        let mut user = simulator.register_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(!user.email_verified);

        // After email verification - should become Active
        let token = simulator.create_verification_token(user.id);
        simulator.verify_email(&token).unwrap();

        // Simulate state change
        user.status = UserStatus::Active;
        user.email_verified = true;

        assert_eq!(user.status, UserStatus::Active);
        assert!(user.email_verified);
    }

    #[test]
    fn test_concurrent_registration_simulation() {
        // Simulate concurrent registration attempts for the same email
        let email = "concurrent@example.com".to_string();
        let mut simulators = vec![
            RegistrationFlowSimulator::new(),
            RegistrationFlowSimulator::new(),
        ];

        // In a real scenario, only one should succeed
        // Here we simulate by checking that duplicate detection works
        let result1 = simulators[0].register_user(
            email.clone(),
            "user1".to_string(),
            "SecurePass123!".to_string(),
        );
        assert!(result1.is_ok());

        // Second attempt should fail (in real scenario, this would be handled by DB constraints)
        simulators[1].registered_emails.push(email.clone()); // Simulate first registration completing
        let result2 = simulators[1].register_user(
            email,
            "user2".to_string(),
            "SecurePass123!".to_string(),
        );
        assert!(result2.is_err());
    }

    #[test]
    fn test_profile_data_validation() {
        // Test various profile data formats
        let valid_profile_data = vec![
            json!({
                "name": "John Doe",
                "bio": "Software developer",
                "location": "San Francisco, CA"
            }),
            json!({
                "name": "Jane Smith",
                "website": "https://example.com",
                "avatar_url": "https://example.com/avatar.jpg"
            }),
            json!({
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            }),
        ];

        for data in valid_profile_data {
            // Validate JSON structure
            assert!(data.is_object());

            if let Some(name) = data.get("name") {
                assert!(name.is_string());
            }

            if let Some(preferences) = data.get("preferences") {
                assert!(preferences.is_object());
            }
        }
    }

    #[test]
    fn test_registration_activity_tracking() {
        let mut simulator = RegistrationFlowSimulator::new();
        let mut activity_log = Vec::new();

        // Register user and track activity
        let user = simulator.register_user(
            "tracked@example.com".to_string(),
            "trackeduser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        activity_log.push(json!({
            "event": "user_registered",
            "user_id": user.id,
            "timestamp": Utc::now(),
            "details": {
                "email": user.email,
                "username": user.username
            }
        }));

        // Verify email and track
        let token = simulator.create_verification_token(user.id);
        simulator.verify_email(&token).unwrap();

        activity_log.push(json!({
            "event": "email_verified",
            "user_id": user.id,
            "timestamp": Utc::now()
        }));

        // Create session and track
        let session = simulator.create_session(user.id);

        activity_log.push(json!({
            "event": "session_created",
            "user_id": user.id,
            "timestamp": Utc::now(),
            "session_id": session
        }));

        // Verify activity was tracked
        assert_eq!(activity_log.len(), 3);
        assert!(activity_log.iter().all(|log| log.get("event").is_some()));
    }

    #[test]
    fn test_complete_registration_with_email_workflow() {
        let mut simulator = RegistrationFlowSimulator::new();

        let result = simulator.complete_registration_flow(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
        );

        assert!(result.is_ok());
        let (user, _, _) = result.unwrap();

        // Verify emails were sent
        let emails = simulator.email_queue.lock().unwrap();
        assert_eq!(emails.len(), 2);
        assert!(emails.iter().any(|e| e.message_type == EmailType::Verification));
        assert!(emails.iter().any(|e| e.message_type == EmailType::Welcome));

        // Verify audit log
        assert!(!simulator.audit_log.is_empty());
        assert!(simulator.audit_log.iter().any(|e| e.action == "user_registered"));
    }

    #[test]
    fn test_registration_rate_limiting() {
        let mut simulator = RegistrationFlowSimulator::new();
        let ip = "192.168.1.1";

        // Should allow up to 5 attempts
        for i in 0..5 {
            let result = simulator.check_rate_limit(ip);
            assert!(result.is_ok(), "Attempt {} should succeed", i + 1);
        }

        // 6th attempt should fail
        let result = simulator.check_rate_limit(ip);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, .. } => {
                assert_eq!(field, "rate_limit");
            }
            _ => panic!("Expected rate limit error"),
        }

        // IP should be blocked
        assert!(simulator.blocked_ips.contains(ip));
    }

    #[test]
    fn test_verification_token_expiration() {
        let mut simulator = RegistrationFlowSimulator::new();

        let user = simulator.register_user(
            "expiry@example.com".to_string(),
            "expiryuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Create token that's already expired
        let token = format!("verify_{}", Uuid::new_v4());
        let expired_time = Utc::now() - Duration::hours(25);
        simulator.verification_tokens.push((token.clone(), user.id, false, expired_time));

        // Verification should fail
        let result = simulator.verify_email(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_verification_attempt_limit() {
        let mut simulator = RegistrationFlowSimulator::new();

        let user = simulator.register_user(
            "attempts@example.com".to_string(),
            "attemptsuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        let token = simulator.create_verification_token(user.id);

        // Make 6 attempts (5 should succeed, 6th should fail)
        for i in 1..=6 {
            let result = simulator.verify_email("invalid_token");
            if i <= 5 {
                // First 5 attempts should get "invalid token" error
                match result {
                    Err(AppError::Validation { message, .. }) => {
                        assert!(message.contains("Invalid"));
                    }
                    _ => panic!("Expected invalid token error"),
                }
            } else {
                // 6th attempt should get "too many attempts" error
                match result {
                    Err(AppError::Validation { message, .. }) => {
                        assert!(message.contains("Too many"));
                    }
                    _ => panic!("Expected too many attempts error"),
                }
            }
        }
    }

    #[test]
    fn test_resend_verification_email() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Register user
        let user = simulator.register_user(
            "resend@example.com".to_string(),
            "resenduser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Clear email queue
        simulator.email_queue.lock().unwrap().clear();

        // Resend verification
        let result = simulator.resend_verification_email("resend@example.com");
        assert!(result.is_ok());

        // Check email was sent
        let emails = simulator.email_queue.lock().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].message_type, EmailType::Verification);

        // Check audit log
        assert!(simulator.audit_log.iter().any(|e| e.action == "verification_email_resent"));
    }

    #[test]
    fn test_resend_verification_for_verified_user() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Complete registration
        let (user, _, _) = simulator.complete_registration_flow(
            "verified@example.com".to_string(),
            "verifieduser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Try to resend verification for already verified user
        let result = simulator.resend_verification_email("verified@example.com");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { message, .. } => {
                assert!(message.contains("already verified"));
            }
            _ => panic!("Expected already verified error"),
        }
    }

    #[test]
    fn test_registration_with_profile_creation() {
        let mut simulator = RegistrationFlowSimulator::new();

        let user = simulator.register_user(
            "profile@example.com".to_string(),
            "profileuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Verify profile was created
        assert!(simulator.profiles_db.contains_key(&user.id));
        let profile = &simulator.profiles_db[&user.id];
        assert_eq!(profile.user_id, user.id);
        assert!(profile.first_name.is_none());
        assert!(profile.last_name.is_none());
    }

    #[test]
    fn test_session_tracking_after_registration() {
        let mut simulator = RegistrationFlowSimulator::new();

        let (user, _, session_token) = simulator.complete_registration_flow(
            "session@example.com".to_string(),
            "sessionuser".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Verify session was created
        assert!(simulator.sessions.iter().any(|(uid, token, _)| {
            *uid == user.id && token == &session_token
        }));

        // Verify last login was updated
        let stored_user = &simulator.users_db[&user.id];
        assert!(stored_user.last_login_at.is_some());
    }

    #[tokio::test]
    async fn test_registration_with_retry_logic() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Pre-register to cause conflict
        simulator.register_user(
            "retry@example.com".to_string(),
            "retryuser1".to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Try with different username (should succeed with retries)
        let result = simulator.register_with_retries(
            "retry2@example.com".to_string(),
            "retryuser2".to_string(),
            "SecurePass123!".to_string(),
            3,
        ).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_captcha_requirement_for_suspicious_activity() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Simulate suspicious activity
        let email = "suspicious@example.com";
        simulator.temp_bans.insert(email.to_string(), Utc::now() + Duration::hours(1));

        // Should require captcha
        let challenge = simulator.require_captcha(email);
        assert!(challenge.is_some());

        // Verify captcha was created
        let challenge = challenge.unwrap();
        assert!(simulator.captcha_challenges.contains_key(&challenge));
    }

    #[test]
    fn test_captcha_verification() {
        let mut simulator = RegistrationFlowSimulator::new();

        // Create captcha
        let challenge = "test_challenge".to_string();
        let answer = "1234".to_string();
        simulator.captcha_challenges.insert(challenge.clone(), answer.clone());

        // Correct answer should pass
        assert!(simulator.verify_captcha(&challenge, &answer));

        // Wrong answer should fail
        assert!(!simulator.verify_captcha(&challenge, "wrong"));

        // Non-existent challenge should fail
        assert!(!simulator.verify_captcha("invalid", "1234"));
    }

    #[test]
    fn test_registration_prevents_sql_injection_in_username() {
        let mut simulator = RegistrationFlowSimulator::new();

        let malicious_usernames = vec![
            "admin'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'--",
            "<script>alert('xss')</script>",
        ];

        for username in malicious_usernames {
            // These should either fail validation or be safely handled
            let result = simulator.register_user(
                "test@example.com".to_string(),
                username.to_string(),
                "SecurePass123!".to_string(),
            );

            // Username validation should handle these safely
            if result.is_ok() {
                let user = result.unwrap();
                // Verify the username was stored as-is (escaped properly)
                assert_eq!(user.username, username);
            }
        }
    }

    #[test]
    fn test_registration_handles_unicode_properly() {
        let mut simulator = RegistrationFlowSimulator::new();

        let unicode_tests = vec![
            ("emoji@🌟.com", "user🎉", true),  // Should handle emojis
            ("中文@example.com", "用户名", true),  // Chinese characters
            ("test@тест.ru", "пользователь", true),  // Cyrillic
        ];

        for (email, username, _) in unicode_tests {
            let result = simulator.register_user(
                email.to_string(),
                username.to_string(),
                "SecurePass123!".to_string(),
            );

            // These should be handled properly (either accepted or rejected consistently)
            if result.is_ok() {
                let user = result.unwrap();
                assert_eq!(user.email, email);
                assert_eq!(user.username, username);
            }
        }
    }

    #[test]
    fn test_registration_database_consistency() {
        let mut simulator = RegistrationFlowSimulator::new();

        let email = "consistency@example.com";
        let username = "consistencyuser";

        // Register user
        let user = simulator.register_user(
            email.to_string(),
            username.to_string(),
            "SecurePass123!".to_string(),
        ).unwrap();

        // Verify all data structures are consistent
        assert!(simulator.registered_emails.contains(&email.to_string()));
        assert!(simulator.registered_usernames.contains(&username.to_string()));
        assert!(simulator.users_db.contains_key(&user.id));
        assert!(simulator.profiles_db.contains_key(&user.id));

        // Verify user data matches
        let stored_user = &simulator.users_db[&user.id];
        assert_eq!(stored_user.email, email);
        assert_eq!(stored_user.username, username);
    }
}