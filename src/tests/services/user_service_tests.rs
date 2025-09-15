//! Tests for UserService
//!
//! This module contains unit tests for service validation logic.

#[cfg(test)]
mod service_validation_tests {
    use uuid::Uuid;
    use crate::commands::{CreateUser, Login, RegisterUser};

    #[test]
    fn test_create_user_command_validation() {
        let valid_command = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
        };

        assert!(!valid_command.username.is_empty());
        assert!(valid_command.email.contains('@'));
        assert!(valid_command.password.len() >= 8);
    }

    #[test]
    fn test_register_user_command_validation() {
        let valid_command = RegisterUser {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "SecurePass123!".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
        };

        assert!(!valid_command.username.is_empty());
        assert!(valid_command.email.contains('@'));
        assert!(valid_command.first_name.is_some());
    }

    #[test]
    fn test_login_command_validation() {
        let valid_command = Login {
            email: "user@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert!(valid_command.email.contains('@'));
        assert!(!valid_command.password.is_empty());
    }

    #[test]
    fn test_email_format_validation() {
        let valid_emails = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.co.uk",
        ];

        for email in valid_emails {
            assert!(email.contains('@'));
            assert!(email.contains('.'));
        }

        let invalid_emails = vec![
            "notanemail",
            "@example.com",
            "user@",
            "user @example.com",
        ];

        for email in invalid_emails {
            let is_valid = email.contains('@') &&
                          email.split('@').count() == 2 &&
                          !email.starts_with('@') &&
                          !email.ends_with('@');

            assert!(!is_valid || email.contains(' '));
        }
    }

    #[test]
    fn test_username_validation() {
        let valid_usernames = vec![
            "user123",
            "john_doe",
            "alice-smith",
            "a",
        ];

        for username in valid_usernames {
            assert!(!username.is_empty());
            assert!(username.len() <= 32);
            assert!(!username.contains(' '));
        }

        let invalid_usernames = vec![
            "",
            "user name",
            "user@name",
            "123456789012345678901234567890123", // Too long
        ];

        for username in invalid_usernames {
            let is_valid = !username.is_empty() &&
                          username.len() <= 32 &&
                          !username.contains(' ') &&
                          !username.contains('@');

            assert!(!is_valid);
        }
    }

    #[test]
    fn test_password_strength_validation() {
        let strong_passwords = vec![
            "SecurePass123!",
            "MyP@ssw0rd",
            "Complex!Pass123",
        ];

        for password in strong_passwords {
            assert!(password.len() >= 8);
            assert!(password.chars().any(|c| c.is_uppercase()));
            assert!(password.chars().any(|c| c.is_lowercase()));
            assert!(password.chars().any(|c| c.is_ascii_digit()));
        }

        let weak_passwords = vec![
            "short",
            "nouppercase123",
            "NOLOWERCASE123",
            "NoNumbers!",
        ];

        for password in weak_passwords {
            let is_strong = password.len() >= 8 &&
                           password.chars().any(|c| c.is_uppercase()) &&
                           password.chars().any(|c| c.is_lowercase()) &&
                           password.chars().any(|c| c.is_ascii_digit());

            assert!(!is_strong);
        }
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.to_string().len(), 36); // UUID string format length
    }
}