//! Tests for User model and validation
//!
//! This module contains comprehensive tests for the User model,
//! including validation, serialization, and business logic.

#[cfg(test)]
mod user_model_tests {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;
    use serde_json;

    use crate::models::{User, UserStatus};
    use crate::errors::AppError;

    // Helper function to create a test user
    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=16384,t=2,p=1$...".to_string(),
            status: UserStatus::Active,
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: Some(Utc::now()),
        }
    }

    #[test]
    fn test_user_creation() {
        let user = create_test_user();

        assert!(!user.id.to_string().is_empty());
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.email_verified);
    }

    #[test]
    fn test_user_status_transitions() {
        let mut user = create_test_user();

        // Test valid status transitions
        user.status = UserStatus::Inactive;
        assert_eq!(user.status, UserStatus::Inactive);

        user.status = UserStatus::Suspended;
        assert_eq!(user.status, UserStatus::Suspended);

        user.status = UserStatus::PendingVerification;
        assert_eq!(user.status, UserStatus::PendingVerification);

        user.status = UserStatus::Active;
        assert_eq!(user.status, UserStatus::Active);
    }

    #[test]
    fn test_user_serialization() {
        let user = create_test_user();

        // Serialize to JSON
        let json = serde_json::to_string(&user).expect("Failed to serialize user");
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"email\":\"test@example.com\""));

        // Deserialize from JSON
        let deserialized: User = serde_json::from_str(&json).expect("Failed to deserialize user");
        assert_eq!(deserialized.username, user.username);
        assert_eq!(deserialized.email, user.email);
        assert_eq!(deserialized.status, user.status);
    }

    #[test]
    fn test_user_validation_email() {
        // Test valid email formats
        let valid_emails = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.co.uk",
            "user123@test-domain.org",
        ];

        for email in valid_emails {
            assert!(validate_email(email), "Email {} should be valid", email);
        }

        // Test invalid email formats
        let invalid_emails = vec![
            "notanemail",
            "@example.com",
            "user@",
            "user@.com",
            "",
        ];

        for email in invalid_emails {
            assert!(!validate_email(email), "Email {} should be invalid", email);
        }

        // Special case for email with space
        let email_with_space = "user @example.com";
        assert!(email_with_space.contains(' '));
        assert!(!validate_email(email_with_space));
    }

    #[test]
    fn test_user_validation_username() {
        // Test valid usernames
        let valid_usernames = vec![
            "user123",
            "john_doe",
            "alice-smith",
            "user.name",
            "a",  // Single character
            "user_123_test",
        ];

        for username in valid_usernames {
            assert!(validate_username(username), "Username {} should be valid", username);
        }

        // Test invalid usernames
        let invalid_usernames = vec![
            "",  // Empty
            "user name",  // Contains space
            "user@name",  // Contains @
            "user#name",  // Contains #
            "123456789012345678901234567890123",  // Too long (>32 chars)
        ];

        for username in invalid_usernames {
            assert!(!validate_username(username), "Username {} should be invalid", username);
        }
    }

    #[test]
    fn test_password_validation() {
        // Test valid passwords
        let valid_passwords = vec![
            "SecurePass123!",
            "MyP@ssw0rd",
            "Complex!Pass123",
            "Test$123456",
            "Valid#Password1",
        ];

        for password in valid_passwords {
            assert!(validate_password(password).is_ok(), "Password {} should be valid", password);
        }

        // Test invalid passwords
        let invalid_passwords = vec![
            "short",  // Too short
            "nouppercase123!",  // No uppercase
            "NOLOWERCASE123!",  // No lowercase
            "NoNumbers!",  // No digits
            "NoSpecialChar123",  // No special characters
            "",  // Empty
            "       ",  // Only spaces
        ];

        for password in invalid_passwords {
            assert!(validate_password(password).is_err(), "Password {} should be invalid", password);
        }
    }


    #[test]
    fn test_user_equality() {
        let user1 = create_test_user();
        let mut user2 = create_test_user();
        user2.id = user1.id;  // Same ID

        assert_eq!(user1.id, user2.id);

        // Different IDs should not be equal
        user2.id = Uuid::new_v4();
        assert_ne!(user1.id, user2.id);
    }

    #[test]
    fn test_user_clone() {
        let user1 = create_test_user();
        let user2 = user1.clone();

        assert_eq!(user1.id, user2.id);
        assert_eq!(user1.username, user2.username);
        assert_eq!(user1.email, user2.email);
        assert_eq!(user1.status, user2.status);
    }


    // Validation helper functions
    fn validate_email(email: &str) -> bool {
        // Simple email validation
        if email.contains(' ') {
            return false;
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }

        let local = parts[0];
        let domain = parts[1];

        !local.is_empty() && !domain.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    }

    fn validate_username(username: &str) -> bool {
        // Username validation rules:
        // - 1-32 characters
        // - Alphanumeric, underscore, hyphen, dot
        // - Cannot be empty
        if username.is_empty() || username.len() > 32 {
            return false;
        }

        username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    }

    fn validate_password(password: &str) -> Result<(), String> {
        // Password validation rules:
        // - Minimum 8 characters
        // - At least one uppercase letter
        // - At least one lowercase letter
        // - At least one digit
        // - At least one special character

        if password.len() < 8 {
            return Err("Password must be at least 8 characters long".to_string());
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            return Err("Password must contain at least one uppercase letter".to_string());
        }

        if !password.chars().any(|c| c.is_lowercase()) {
            return Err("Password must contain at least one lowercase letter".to_string());
        }

        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err("Password must contain at least one digit".to_string());
        }

        if !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err("Password must contain at least one special character".to_string());
        }

        Ok(())
    }

}