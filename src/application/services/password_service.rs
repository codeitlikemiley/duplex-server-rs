use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{errors::AppError, PostgreSQL};

pub struct PasswordService {
    db: PostgreSQL,
}

impl PasswordService {
    pub fn new(db: PostgreSQL) -> Self {
        Self { db }
    }

    /// Validate password strength
    pub fn validate_password_strength(password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::Validation {
                field: "password".to_string(),
                message: "Password must be at least 8 characters long".to_string(),
            });
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_uppercase || !has_lowercase || !has_digit {
            return Err(AppError::Validation {
                field: "password".to_string(),
                message: "Password must contain uppercase, lowercase, and numbers".to_string(),
            });
        }

        if !has_special {
            return Err(AppError::Validation {
                field: "password".to_string(),
                message: "Password must contain at least one special character".to_string(),
            });
        }

        Ok(())
    }

    /// Hash a password
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::Internal {
                message: "Failed to hash password".to_string(),
            })
    }

    /// Verify a password against its hash
    pub fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
        let argon2 = Argon2::default();
        let parsed_hash = argon2::password_hash::PasswordHash::new(hash)
            .map_err(|_| AppError::Internal {
                message: "Invalid password hash format".to_string(),
            })?;

        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Authentication {
                message: "Invalid password".to_string(),
            })
    }

    /// Change user password
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        // Validate new password strength
        Self::validate_password_strength(new_password)?;

        // Fetch current password hash
        let user = sqlx::query!(
            r#"
            SELECT password_hash
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch user: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "User".to_string(),
            id: Some(user_id.to_string()),
        })?;

        // Verify current password
        Self::verify_password(current_password, &user.password_hash)?;

        // Hash new password
        let new_password_hash = Self::hash_password(new_password)?;

        // Update password in database
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $2,
                updated_at = $3
            WHERE id = $1
            "#,
            user_id,
            new_password_hash,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update password: {}", e),
        })?;

        tracing::info!("Password changed successfully for user: {}", user_id);
        Ok(())
    }

    /// Reset password (for forgot password flow)
    pub async fn reset_password(
        &self,
        user_id: Uuid,
        new_password: &str,
    ) -> Result<(), AppError> {
        // Validate new password strength
        Self::validate_password_strength(new_password)?;

        // Hash new password
        let new_password_hash = Self::hash_password(new_password)?;

        // Update password in database
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $2,
                updated_at = $3
            WHERE id = $1
            "#,
            user_id,
            new_password_hash,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to reset password: {}", e),
        })?;

        // Revoke all existing sessions to force re-login
        sqlx::query!(
            r#"
            UPDATE user_sessions
            SET revoked = true
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to revoke sessions: {}", e),
        })?;

        tracing::info!("Password reset successfully for user: {}", user_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_password_strength_valid() {
        let valid_passwords = vec![
            "Password123!",
            "MySecure#Pass1",
            "Str0ng@Password",
            "Complex$123pass",
            "Valid1!password",
        ];

        for password in valid_passwords {
            assert!(PasswordService::validate_password_strength(password).is_ok(),
                    "Password '{}' should be valid", password);
        }
    }

    #[test]
    fn test_validate_password_strength_too_short() {
        let short_passwords = vec![
            "Pass1!",
            "123",
            "Ab1!",
            "",
            "1234567",
        ];

        for password in short_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid (too short)", password);
            match result.unwrap_err() {
                AppError::Validation { field, message } => {
                    assert_eq!(field, "password");
                    assert!(message.contains("at least 8 characters"));
                }
                _ => panic!("Expected validation error for short password"),
            }
        }
    }

    #[test]
    fn test_validate_password_strength_missing_uppercase() {
        let no_uppercase_passwords = vec![
            "password123!",
            "lowercase1!",
            "mypass123@",
            "secret456#",
        ];

        for password in no_uppercase_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid (no uppercase)", password);
            match result.unwrap_err() {
                AppError::Validation { field, message } => {
                    assert_eq!(field, "password");
                    assert!(message.contains("uppercase"));
                }
                _ => panic!("Expected validation error for no uppercase"),
            }
        }
    }

    #[test]
    fn test_validate_password_strength_missing_lowercase() {
        let no_lowercase_passwords = vec![
            "PASSWORD123!",
            "UPPERCASE1!",
            "MYPASS123@",
            "SECRET456#",
        ];

        for password in no_lowercase_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid (no lowercase)", password);
            match result.unwrap_err() {
                AppError::Validation { field, message } => {
                    assert_eq!(field, "password");
                    assert!(message.contains("lowercase"));
                }
                _ => panic!("Expected validation error for no lowercase"),
            }
        }
    }

    #[test]
    fn test_validate_password_strength_missing_digit() {
        let no_digit_passwords = vec![
            "Password!",
            "MySecure#Pass",
            "Strong@Password",
            "Complex$pass",
        ];

        for password in no_digit_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid (no digit)", password);
            match result.unwrap_err() {
                AppError::Validation { field, message } => {
                    assert_eq!(field, "password");
                    assert!(message.contains("numbers"));
                }
                _ => panic!("Expected validation error for no digit"),
            }
        }
    }

    #[test]
    fn test_validate_password_strength_missing_special() {
        let no_special_passwords = vec![
            "Password123",
            "MySecurePass1",
            "StrongPassword5",
            "Complexpass9",
        ];

        for password in no_special_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid (no special char)", password);
            match result.unwrap_err() {
                AppError::Validation { field, message } => {
                    assert_eq!(field, "password");
                    assert!(message.contains("special character"));
                }
                _ => panic!("Expected validation error for no special character"),
            }
        }
    }

    #[test]
    fn test_validate_password_strength_multiple_issues() {
        let invalid_passwords = vec![
            "pass",      // too short, no uppercase, no digit, no special
            "PASSWORD",  // no lowercase, no digit, no special
            "password",  // no uppercase, no digit, no special
            "12345678",  // no uppercase, no lowercase, no special
            "Password",  // no digit, no special
        ];

        for password in invalid_passwords {
            let result = PasswordService::validate_password_strength(password);
            assert!(result.is_err(), "Password '{}' should be invalid", password);
        }
    }

    #[test]
    fn test_validate_password_strength_edge_cases() {
        // Exactly 8 characters with all requirements
        assert!(PasswordService::validate_password_strength("Pass123!").is_ok());

        // Unicode characters count as special
        assert!(PasswordService::validate_password_strength("Passwörد123").is_ok());

        // Emojis count as special characters
        assert!(PasswordService::validate_password_strength("Password1😀").is_ok());
    }

    #[test]
    fn test_hash_password_creates_valid_hash() {
        let password = "TestPassword123!";
        let result = PasswordService::hash_password(password);

        assert!(result.is_ok());
        let hash = result.unwrap();

        // Argon2 hashes start with $argon2
        assert!(hash.starts_with("$argon2"));
        assert!(hash.len() > 50); // Reasonable hash length
    }

    #[test]
    fn test_hash_password_different_hashes_for_same_password() {
        let password = "TestPassword123!";
        let hash1 = PasswordService::hash_password(password).unwrap();
        let hash2 = PasswordService::hash_password(password).unwrap();

        // Should generate different hashes due to random salt
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_password_empty_string() {
        let result = PasswordService::hash_password("");
        assert!(result.is_ok()); // Hashing should succeed even for empty string
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "测试密码123!";
        let result = PasswordService::hash_password(password);
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "TestPassword123!";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "TestPassword123!";
        let wrong_password = "WrongPassword123!";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(wrong_password, &hash);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert_eq!(message, "Invalid password");
            }
            _ => panic!("Expected authentication error"),
        }
    }

    #[test]
    fn test_verify_password_invalid_hash_format() {
        let password = "TestPassword123!";
        let invalid_hash = "not_a_valid_hash";

        let result = PasswordService::verify_password(password, invalid_hash);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Internal { message } => {
                assert_eq!(message, "Invalid password hash format");
            }
            _ => panic!("Expected internal error"),
        }
    }

    #[test]
    fn test_verify_password_empty_password() {
        let password = "";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(password, &hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_password_case_sensitive() {
        let password = "TestPassword123!";
        let wrong_case = "testpassword123!";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(wrong_case, &hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_unicode() {
        let password = "测试密码123!";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(password, &hash);
        assert!(result.is_ok());

        // Wrong unicode should fail
        let wrong_unicode = "测试密碼123!"; // Different Chinese characters
        let result = PasswordService::verify_password(wrong_unicode, &hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_whitespace_sensitive() {
        let password = "TestPassword123!";
        let password_with_space = " TestPassword123!";
        let password_with_trailing = "TestPassword123! ";
        let hash = PasswordService::hash_password(password).unwrap();

        // Should fail with extra whitespace
        assert!(PasswordService::verify_password(password_with_space, &hash).is_err());
        assert!(PasswordService::verify_password(password_with_trailing, &hash).is_err());
    }

    #[test]
    fn test_special_characters_in_password_validation() {
        let special_chars = vec![
            "!@#$%^&*()",
            "[]{}|;':\",./<>?",
            "`~-_=+",
            "¡¢£¤¥¦§¨©ª«¬®¯°±²³",
        ];

        for special in special_chars {
            let password = format!("Password1{}", special.chars().next().unwrap());
            assert!(
                PasswordService::validate_password_strength(&password).is_ok(),
                "Password with special character '{}' should be valid", special.chars().next().unwrap()
            );
        }
    }

    #[test]
    fn test_long_password_validation() {
        let long_password = "A".repeat(100) + "a1!";
        assert!(PasswordService::validate_password_strength(&long_password).is_ok());

        let very_long_password = "A".repeat(1000) + "a1!";
        assert!(PasswordService::validate_password_strength(&very_long_password).is_ok());
    }

    #[test]
    fn test_minimal_valid_password() {
        // Shortest possible valid password
        let minimal = "Aa1!bcde"; // 8 chars, has upper, lower, digit, special
        assert!(PasswordService::validate_password_strength(minimal).is_ok());
    }
}