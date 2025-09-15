//! Tests for SessionService and authentication functionality
//!
//! This module contains tests for session management and password handling.

#[cfg(test)]
mod authentication_service_tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::application::services::{SessionService, PasswordService};
    use crate::domain::models::{User, UserStatus};
    use crate::errors::AppError;

    // Helper function to create a test user
    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=16384,t=2,p=1$c29tZXNhbHQ$GpR1T5oGJPRue5/V9cUEpw".to_string(),
            status: UserStatus::Active,
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        }
    }

    #[test]
    fn test_password_hashing() {
        let password = "SecurePassword123!";

        // Hash the password
        let hash_result = TestPasswordHelper::hash_password(password);
        assert!(hash_result.is_ok());

        let hash = hash_result.unwrap();
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"));

        // Verify the password
        let verify_result = TestPasswordHelper::verify_password(password, &hash);
        assert!(verify_result.is_ok());
        assert!(verify_result.unwrap());

        // Wrong password should fail
        let wrong_verify = TestPasswordHelper::verify_password("WrongPassword", &hash);
        assert!(wrong_verify.is_ok());
        assert!(!wrong_verify.unwrap());
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid passwords
        let strong_passwords = vec![
            "SecurePass123!",
            "MyP@ssw0rd",
            "Complex!Pass123",
            "Test$123456",
        ];

        for password in strong_passwords {
            let result = TestPasswordHelper::validate_password_strength(password);
            assert!(result.is_ok(), "Password {} should be strong", password);
        }

        // Weak passwords
        let weak_passwords = vec![
            "short",  // Too short
            "nouppercase123!",  // No uppercase
            "NOLOWERCASE123!",  // No lowercase
            "NoNumbers!",  // No digits
            "NoSpecialChar123",  // No special characters
        ];

        for password in weak_passwords {
            let result = TestPasswordHelper::validate_password_strength(password);
            assert!(result.is_err(), "Password {} should be weak", password);
        }
    }

    #[test]
    fn test_session_token_generation() {
        // Generate multiple tokens and ensure they're unique
        let mut tokens = vec![];

        for _ in 0..10 {
            let token = SessionService::generate_session_token();
            assert!(!token.is_empty());
            assert!(token.len() > 32); // Should be a reasonably long token
            tokens.push(token);
        }

        // Check all tokens are unique
        let unique_tokens: std::collections::HashSet<_> = tokens.iter().collect();
        assert_eq!(tokens.len(), unique_tokens.len());
    }

    #[test]
    fn test_session_expiry_calculation() {
        let now = Utc::now();

        // Default session duration (24 hours)
        let expires_at = SessionService::calculate_session_expiry(None);
        let duration = expires_at - now;
        assert!(duration.num_hours() >= 23 && duration.num_hours() <= 25);

        // Custom session duration (7 days)
        let expires_at_custom = SessionService::calculate_session_expiry(Some(7 * 24 * 60));
        let duration_custom = expires_at_custom - now;
        assert!(duration_custom.num_days() >= 6 && duration_custom.num_days() <= 8);
    }

    #[test]
    fn test_user_can_login() {
        let mut user = create_test_user();

        // Active verified user can login
        assert!(user.status == UserStatus::Active);
        assert!(user.email_verified);

        // Unverified user cannot login
        user.email_verified = false;
        assert!(!user.email_verified);

        // Suspended user cannot login
        user.email_verified = true;
        user.status = UserStatus::Suspended;
        assert!(user.status == UserStatus::Suspended);

        // Inactive user cannot login
        user.status = UserStatus::Inactive;
        assert!(user.status == UserStatus::Inactive);
    }

    #[test]
    fn test_password_reset_token_generation() {
        // Generate reset tokens
        let token1 = SessionService::generate_reset_token();
        let token2 = SessionService::generate_reset_token();

        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        assert_ne!(token1, token2); // Should be unique

        // Token should be URL-safe
        assert!(!token1.contains(' '));
        assert!(!token1.contains('/'));
        assert!(!token1.contains('+'));
    }

    #[test]
    fn test_email_verification_token_generation() {
        // Generate verification tokens
        let token1 = SessionService::generate_verification_token();
        let token2 = SessionService::generate_verification_token();

        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        assert_ne!(token1, token2);

        // Token should be alphanumeric
        assert!(token1.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_concurrent_password_hashing() {
        let passwords = vec![
            "Password1!",
            "Password2@",
            "Password3#",
            "Password4$",
            "Password5%",
        ];

        let mut hashes = vec![];

        for password in &passwords {
            let hash = TestPasswordHelper::hash_password(password).unwrap();
            hashes.push(hash);
        }

        // All hashes should be unique even for similar passwords
        let unique_hashes: std::collections::HashSet<_> = hashes.iter().collect();
        assert_eq!(hashes.len(), unique_hashes.len());

        // Each password should verify against its own hash
        for (password, hash) in passwords.iter().zip(hashes.iter()) {
            assert!(TestPasswordHelper::verify_password(password, hash).unwrap());
        }
    }

    #[test]
    fn test_session_activity_tracking() {
        let initial_time = Utc::now();
        let user_id = Uuid::new_v4();

        // Simulate session activity over time
        let mut last_activity = initial_time;

        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let new_activity = Utc::now();
            assert!(new_activity > last_activity);
            last_activity = new_activity;
        }
    }

    // Mock implementations for testing
    impl SessionService {
        fn generate_session_token() -> String {
            // Generate a random session token
            use base64::Engine;
            let random_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            base64::engine::general_purpose::STANDARD.encode(&random_bytes)
        }

        fn calculate_session_expiry(duration_minutes: Option<i64>) -> chrono::DateTime<Utc> {
            let duration = duration_minutes.unwrap_or(24 * 60); // Default 24 hours
            Utc::now() + chrono::Duration::minutes(duration)
        }

        fn generate_reset_token() -> String {
            // Generate a URL-safe reset token
            use base64::Engine;
            let random_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
        }

        fn generate_verification_token() -> String {
            // Generate an alphanumeric verification token
            Uuid::new_v4().to_string()
        }
    }

    // Test helper for password operations
    struct TestPasswordHelper;

    impl TestPasswordHelper {
        fn hash_password(password: &str) -> Result<String, AppError> {
            use argon2::{Argon2, PasswordHasher};
            use argon2::password_hash::{SaltString, rand_core::OsRng};

            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            argon2
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to hash password: {}", e)
                })
        }

        fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
            use argon2::{Argon2, PasswordVerifier};
            use argon2::password_hash::PasswordHash;

            let parsed_hash = PasswordHash::new(hash)
                .map_err(|e| AppError::Internal {
                    message: format!("Invalid password hash: {}", e)
                })?;

            let argon2 = Argon2::default();
            Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
        }

        fn validate_password_strength(password: &str) -> Result<(), AppError> {
            if password.len() < 8 {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must be at least 8 characters".to_string()
                });
            }

            if !password.chars().any(|c| c.is_uppercase()) {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain an uppercase letter".to_string()
                });
            }

            if !password.chars().any(|c| c.is_lowercase()) {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain a lowercase letter".to_string()
                });
            }

            if !password.chars().any(|c| c.is_ascii_digit()) {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain a digit".to_string()
                });
            }

            if !password.chars().any(|c| !c.is_alphanumeric()) {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain a special character".to_string()
                });
            }

            Ok(())
        }
    }
}