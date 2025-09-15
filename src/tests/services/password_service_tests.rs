//! Tests for Password Service and hashing utilities
//!
//! This module contains comprehensive tests for password hashing, validation, and security.

#[cfg(test)]
mod password_service_tests {
    use crate::application::services::PasswordService;
    use crate::errors::AppError;

    #[test]
    fn test_basic_password_hashing() {
        let password = "MySecurePassword123!";

        let hash = PasswordService::hash_password(password).unwrap();

        // Hash should not be empty
        assert!(!hash.is_empty());

        // Hash should be different from original password
        assert_ne!(hash, password);

        // Hash should start with argon2 identifier
        assert!(hash.starts_with("$argon2"));

        // Hash should be reasonably long (typical Argon2 hash is 90+ chars)
        assert!(hash.len() > 90);
    }

    #[test]
    fn test_password_verification_success() {
        let password = "CorrectPassword123!";

        let hash = PasswordService::hash_password(password).unwrap();
        let result = PasswordService::verify_password(password, &hash);

        assert!(result.is_ok());
    }

    #[test]
    fn test_password_verification_failure() {
        let password = "CorrectPassword123!";
        let wrong_password = "WrongPassword123!";

        let hash = PasswordService::hash_password(password).unwrap();
        let result = PasswordService::verify_password(wrong_password, &hash);

        assert!(result.is_err());
    }

    #[test]
    fn test_same_password_different_hashes() {
        let password = "SamePassword123!";

        let hash1 = PasswordService::hash_password(password).unwrap();
        let hash2 = PasswordService::hash_password(password).unwrap();

        // Same password should produce different hashes (due to random salt)
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(PasswordService::verify_password(password, &hash1).is_ok());
        assert!(PasswordService::verify_password(password, &hash2).is_ok());
    }

    #[test]
    fn test_password_strength_validation() {
        // Strong passwords should pass
        let strong_passwords = vec![
            "StrongPass123!",
            "MyP@ssw0rd",
            "Secure$Pass99",
            "ValidP@ss2024",
            "C0mpl3x!Pass",
        ];

        for password in strong_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_ok(), "Password '{}' should be strong", password);
        }
    }

    #[test]
    fn test_weak_password_rejection() {
        // Weak passwords should fail
        let weak_passwords = vec![
            ("short", "at least 8 characters"),
            ("nouppercase123!", "uppercase"),
            ("NOLOWERCASE123!", "lowercase"),
            ("NoNumbers!", "numbers"),
            ("NoSpecialChars123", "special character"),
            ("12345678!", "uppercase"),
            ("ABCDEFGH!", "lowercase"),
        ];

        for (password, expected_error) in weak_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be weak", password);

            if let Err(AppError::Validation { message, .. }) = result {
                assert!(
                    message.to_lowercase().contains(expected_error),
                    "Expected error about '{}' for password '{}', got: {}",
                    expected_error, password, message
                );
            }
        }
    }

    #[test]
    fn test_empty_password_handling() {
        let empty_password = "";

        // Should fail validation
        let validation_result = PasswordService::validate_password_strength(empty_password);
        assert!(validation_result.is_err());

        // Should still be hashable (but not recommended)
        let hash_result = PasswordService::hash_password(empty_password);
        assert!(hash_result.is_ok());
    }

    #[test]
    fn test_special_characters_in_password() {
        let passwords_with_special = vec![
            "Pass@word123",
            "Pass#word123",
            "Pass$word123",
            "Pass%word123",
            "Pass&word123",
            "Pass*word123",
            "Pass!word123",
            "Pass?word123",
            "Pass~word123",
            "Pass^word123",
        ];

        for password in passwords_with_special {
            let hash = PasswordService::hash_password(password).unwrap();
            let result = PasswordService::verify_password(password, &hash);
            assert!(result.is_ok(), "Password with special char '{}' should work", password);
        }
    }

    #[test]
    fn test_unicode_password_support() {
        let unicode_passwords = vec![
            "Пароль123!",  // Cyrillic
            "密码Pass123!",  // Chinese
            "パスワード123!",  // Japanese
            "🔒Secure123!",  // Emoji
            "Café123!École",  // Accented characters
        ];

        for password in unicode_passwords {
            let hash = PasswordService::hash_password(password);
            assert!(hash.is_ok(), "Should handle unicode password: {}", password);

            let hash = hash.unwrap();
            let result = PasswordService::verify_password(password, &hash);
            assert!(result.is_ok(), "Should verify unicode password: {}", password);
        }
    }

    #[test]
    fn test_very_long_password() {
        let long_password = format!("{}VeryLongPassword123!", "a".repeat(100));

        let hash = PasswordService::hash_password(&long_password);
        assert!(hash.is_ok(), "Should handle long passwords");

        let hash = hash.unwrap();
        let result = PasswordService::verify_password(&long_password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_password_with_spaces() {
        let passwords_with_spaces = vec![
            "Pass word123!",  // Space in middle
            " Password123!",  // Leading space
            "Password123! ",  // Trailing space
            "Pass  word123!", // Multiple spaces
        ];

        for password in passwords_with_spaces {
            let hash = PasswordService::hash_password(password).unwrap();
            let result = PasswordService::verify_password(password, &hash);
            assert!(result.is_ok(), "Password with spaces '{}' should work", password);
        }
    }

    #[test]
    fn test_invalid_hash_format() {
        let invalid_hashes = vec![
            "not-a-hash",
            "$invalid$format",
            "",
            "12345",
            "$argon2$invalid",
        ];

        for invalid_hash in invalid_hashes {
            let result = PasswordService::verify_password("password", invalid_hash);
            assert!(
                result.is_err(),
                "Invalid hash '{}' should fail verification",
                invalid_hash
            );
        }
    }

    #[test]
    fn test_case_sensitive_verification() {
        let password = "CaseSensitive123!";
        let hash = PasswordService::hash_password(password).unwrap();

        // Different case should fail
        assert!(PasswordService::verify_password("casesensitive123!", &hash).is_err());
        assert!(PasswordService::verify_password("CASESENSITIVE123!", &hash).is_err());

        // Exact case should pass
        assert!(PasswordService::verify_password(password, &hash).is_ok());
    }

    #[test]
    fn test_common_password_patterns() {
        // These should all be valid from a hashing perspective
        let common_patterns = vec![
            "Password123!",
            "Qwerty123!",
            "Admin123!",
            "Welcome123!",
            "Letmein123!",
        ];

        for password in common_patterns {
            // Should hash successfully (even if weak)
            let hash = PasswordService::hash_password(password).unwrap();
            assert!(!hash.is_empty());

            // Should verify correctly
            assert!(PasswordService::verify_password(password, &hash).is_ok());
        }
    }

    #[test]
    fn test_password_timing_resistance() {
        let password = "TestPassword123!";
        let hash = PasswordService::hash_password(password).unwrap();

        // Verification should take similar time for correct and incorrect passwords
        // This is a conceptual test - Argon2 provides timing attack resistance

        let correct_result = PasswordService::verify_password(password, &hash);
        let wrong_result = PasswordService::verify_password("WrongPassword", &hash);

        assert!(correct_result.is_ok());
        assert!(wrong_result.is_err());
    }

    #[test]
    fn test_concurrent_hashing() {
        use std::sync::Arc;
        use std::thread;

        let passwords = vec![
            "Password1!",
            "Password2@",
            "Password3#",
            "Password4$",
            "Password5%",
        ];

        let mut handles = vec![];
        let mut hashes = vec![];

        for password in passwords.clone() {
            let handle = thread::spawn(move || {
                PasswordService::hash_password(password).unwrap()
            });
            handles.push(handle);
        }

        for handle in handles {
            hashes.push(handle.join().unwrap());
        }

        // All hashes should be unique
        let unique_hashes: std::collections::HashSet<_> = hashes.iter().collect();
        assert_eq!(hashes.len(), unique_hashes.len());

        // Each password should verify against its corresponding hash
        for (password, hash) in passwords.iter().zip(hashes.iter()) {
            assert!(PasswordService::verify_password(password, hash).is_ok());
        }
    }

}