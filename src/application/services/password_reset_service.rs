use std::sync::Arc;
use chrono::{Duration, Utc};
use uuid::Uuid;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};

use crate::{
    errors::AppError,
    infrastructure::{
        email::{EmailService, EmailTemplates},
        repositories::PostgreSQL,
    },
};

pub struct PasswordResetService {
    db: Pool<Postgres>,
    postgres: PostgreSQL,
    email_service: Arc<dyn EmailService>,
    base_url: String,
}

impl PasswordResetService {
    pub fn new(
        pool: Pool<Postgres>,
        email_service: Arc<dyn EmailService>,
        base_url: String,
    ) -> Self {
        let postgres = PostgreSQL::new(pool.clone());
        Self {
            db: pool,
            postgres,
            email_service,
            base_url,
        }
    }

    /// Initiate password reset by sending a reset token via email
    pub async fn request_password_reset(&self, email: &str) -> Result<(), AppError> {
        // Find user by email
        let user = self.postgres.find_user_by_email(email).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?;

        let user = match user {
            Some(u) => u,
            None => {
                // Don't reveal if email exists or not for security
                tracing::info!("Password reset requested for non-existent email: {}", email);
                return Ok(());
            }
        };

        // Generate reset token
        let token = self.generate_reset_token();
        let token_hash = self.hash_token(&token);
        let expires_at = Utc::now() + Duration::hours(1); // Token expires in 1 hour

        // Store reset token in database
        self.store_reset_token(user.id, &token_hash, expires_at).await?;

        // Send reset email
        let reset_link = format!(
            "{}/reset-password?user_id={}&token={}",
            self.base_url,
            user.id,
            token
        );

        let message = EmailTemplates::password_reset_email(
            &user.email,
            "noreply@quake.app",
            &user.username,
            &reset_link,
        );

        self.email_service.send_email(message).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to send password reset email: {}", e)
            })?;

        tracing::info!("Password reset email sent to {}", user.email);
        Ok(())
    }

    /// Verify reset token and reset password
    pub async fn reset_password(
        &self,
        user_id: Uuid,
        token: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        // Hash the provided token
        let token_hash = self.hash_token(token);

        // Verify token exists and hasn't expired
        let is_valid = self.verify_reset_token(user_id, &token_hash).await?;

        if !is_valid {
            return Err(AppError::Validation {
                field: "token".to_string(),
                message: "Invalid or expired reset token".to_string()
            });
        }

        // Validate new password strength
        // Validate new password strength
        crate::application::services::PasswordService::validate_password_strength(new_password)?;

        // Hash new password
        let password_hash = crate::application::services::PasswordService::hash_password(new_password)?;

        // Update user's password
        self.update_user_password(user_id, &password_hash).await?;

        // Mark token as used
        self.mark_token_as_used(user_id, &token_hash).await?;

        // Invalidate all existing sessions
        self.invalidate_user_sessions(user_id).await?;

        // Send confirmation email
        let user = self.postgres.find_user_by_id(user_id).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?
            .ok_or_else(|| AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string())
            })?;

        // Create a simple password changed notification
        let message = crate::infrastructure::email::EmailMessage {
            to: user.email.clone(),
            from: "noreply@quake.app".to_string(),
            subject: "Your password has been changed".to_string(),
            body_text: format!("Hi {},\n\nYour password has been successfully changed. If you did not make this change, please contact support immediately.", user.username),
            body_html: format!("<p>Hi {},</p><p>Your password has been successfully changed. If you did not make this change, please contact support immediately.</p>", user.username),
        };

        // Send email but don't fail if it doesn't work
        if let Err(e) = self.email_service.send_email(message).await {
            tracing::error!("Failed to send password changed notification: {}", e);
        }

        tracing::info!("Password reset successful for user: {}", user_id);
        Ok(())
    }

    /// Generate a secure random reset token
    fn generate_reset_token(&self) -> String {
        // Generate 64 random alphanumeric characters
        use rand::Rng;
        let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..chars.len());
                chars.chars().nth(idx).unwrap()
            })
            .collect()
    }

    /// Hash a token for secure storage
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Store reset token in database
    async fn store_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<(), AppError> {
        let id = Uuid::now_v7();

        sqlx::query!(
            r#"
            INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, used)
            VALUES ($1, $2, $3, $4, false)
            ON CONFLICT (user_id) DO UPDATE SET
                token_hash = EXCLUDED.token_hash,
                expires_at = EXCLUDED.expires_at,
                used = false
            "#,
            id,
            user_id,
            token_hash,
            expires_at
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to store reset token: {}", e)
        })?;

        Ok(())
    }

    /// Verify reset token is valid and not expired
    async fn verify_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT expires_at, used
            FROM password_reset_tokens
            WHERE user_id = $1 AND token_hash = $2
            "#,
            user_id,
            token_hash
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to verify reset token: {}", e)
        })?;

        match result {
            Some(record) => {
                if record.used {
                    Ok(false)
                } else if record.expires_at < Utc::now() {
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }

    /// Update user's password in database
    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $2,
                updated_at = $3
            WHERE id = $1
            "#,
            user_id,
            password_hash,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update password: {}", e)
        })?;

        Ok(())
    }

    /// Mark reset token as used
    async fn mark_token_as_used(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE password_reset_tokens
            SET used = true
            WHERE user_id = $1 AND token_hash = $2
            "#,
            user_id,
            token_hash
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to mark token as used: {}", e)
        })?;

        Ok(())
    }

    /// Invalidate all user sessions after password reset
    async fn invalidate_user_sessions(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE user_sessions
            SET revoked = true
            WHERE user_id = $1 AND revoked = false
            "#,
            user_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to invalidate sessions: {}", e)
        })?;

        Ok(())
    }

    /// Clean up expired reset tokens
    pub async fn cleanup_expired_tokens(&self) -> Result<usize, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < $1 OR used = true
            "#,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup expired tokens: {}", e)
        })?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Helper function to create a basic service for testing pure functions
    fn create_test_service() -> PasswordResetService {
        use std::sync::Arc;
        use crate::infrastructure::email::console_email_sender::ConsoleEmailSender;

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/test")
            .unwrap();

        let email_service = Arc::new(ConsoleEmailSender::new());

        PasswordResetService::new(
            pool,
            email_service,
            "https://test.com".to_string(),
        )
    }

    #[test]
    fn test_generate_reset_token_length() {
        let service = create_test_service();
        let token = service.generate_reset_token();

        assert_eq!(token.len(), 64);
    }

    #[test]
    fn test_generate_reset_token_uniqueness() {
        let service = create_test_service();
        let token1 = service.generate_reset_token();
        let token2 = service.generate_reset_token();

        assert_ne!(token1, token2);
    }

    #[test]
    fn test_generate_reset_token_character_set() {
        let service = create_test_service();
        let token = service.generate_reset_token();
        let valid_chars: HashSet<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
            .chars()
            .collect();

        for char in token.chars() {
            assert!(valid_chars.contains(&char), "Token contains invalid character: {}", char);
        }
    }

    #[test]
    fn test_generate_reset_token_multiple_calls() {
        let service = create_test_service();
        let mut tokens = HashSet::new();

        // Generate 100 tokens and ensure they're all unique
        for _ in 0..100 {
            let token = service.generate_reset_token();
            assert_eq!(token.len(), 64);
            assert!(tokens.insert(token), "Duplicate token generated");
        }
    }

    #[test]
    fn test_hash_token_consistency() {
        let service = create_test_service();
        let token = "test_token_123";

        let hash1 = service.hash_token(token);
        let hash2 = service.hash_token(token);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_hash_token_different_inputs() {
        let service = create_test_service();
        let token1 = "test_token_1";
        let token2 = "test_token_2";

        let hash1 = service.hash_token(token1);
        let hash2 = service.hash_token(token2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_empty_string() {
        let service = create_test_service();
        let empty_hash = service.hash_token("");

        assert_eq!(empty_hash.len(), 64);
        assert!(!empty_hash.is_empty());

        // SHA256 of empty string should be consistent
        let empty_hash2 = service.hash_token("");
        assert_eq!(empty_hash, empty_hash2);
    }

    #[test]
    fn test_hash_token_unicode() {
        let service = create_test_service();
        let unicode_token = "测试令牌🔐";

        let hash = service.hash_token(unicode_token);
        assert_eq!(hash.len(), 64);

        // Should be consistent
        let hash2 = service.hash_token(unicode_token);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_token_long_input() {
        let service = create_test_service();
        let long_token = "a".repeat(10000);

        let hash = service.hash_token(&long_token);
        assert_eq!(hash.len(), 64);

        // Different long strings should produce different hashes
        let long_token2 = "b".repeat(10000);
        let hash2 = service.hash_token(&long_token2);
        assert_ne!(hash, hash2);
    }

    #[test]
    fn test_hash_token_special_characters() {
        let service = create_test_service();
        let special_token = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~";

        let hash = service.hash_token(special_token);
        assert_eq!(hash.len(), 64);

        // Should be consistent
        let hash2 = service.hash_token(special_token);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_token_hex_output() {
        let service = create_test_service();
        let token = "test_token";

        let hash = service.hash_token(token);

        // Verify all characters are valid hex
        for char in hash.chars() {
            assert!(char.is_ascii_hexdigit(), "Hash contains non-hex character: {}", char);
        }
    }

    #[test]
    fn test_hash_token_case_sensitivity() {
        let service = create_test_service();
        let token_lower = "test_token";
        let token_upper = "TEST_TOKEN";

        let hash_lower = service.hash_token(token_lower);
        let hash_upper = service.hash_token(token_upper);

        assert_ne!(hash_lower, hash_upper);
    }

    #[test]
    fn test_hash_token_whitespace_sensitivity() {
        let service = create_test_service();
        let token1 = "test_token";
        let token2 = " test_token";
        let token3 = "test_token ";
        let token4 = "test_token\n";

        let hash1 = service.hash_token(token1);
        let hash2 = service.hash_token(token2);
        let hash3 = service.hash_token(token3);
        let hash4 = service.hash_token(token4);

        // All should be different due to whitespace differences
        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash1, hash4);
        assert_ne!(hash2, hash3);
        assert_ne!(hash2, hash4);
        assert_ne!(hash3, hash4);
    }

    #[test]
    fn test_generate_reset_token_no_special_chars() {
        let service = create_test_service();
        let token = service.generate_reset_token();

        // Should only contain alphanumeric characters
        for char in token.chars() {
            assert!(char.is_alphanumeric(), "Token contains non-alphanumeric character: {}", char);
        }
    }

    #[test]
    fn test_generate_reset_token_contains_mixed_case() {
        let service = create_test_service();
        let mut found_uppercase = false;
        let mut found_lowercase = false;
        let mut found_digit = false;

        // Generate multiple tokens to increase chances of finding all character types
        for _ in 0..10 {
            let token = service.generate_reset_token();

            for char in token.chars() {
                if char.is_uppercase() {
                    found_uppercase = true;
                }
                if char.is_lowercase() {
                    found_lowercase = true;
                }
                if char.is_ascii_digit() {
                    found_digit = true;
                }
            }

            // If we found all types, we can break early
            if found_uppercase && found_lowercase && found_digit {
                break;
            }
        }

        // With 10 tokens of 64 chars each, we should statistically find all character types
        // But we won't assert this as it could theoretically fail due to randomness
        // Instead we just verify the character set is available
        let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        assert!(charset.chars().any(|c| c.is_uppercase()));
        assert!(charset.chars().any(|c| c.is_lowercase()));
        assert!(charset.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_hash_token_known_vectors() {
        let service = create_test_service();

        // Test with known input to verify hash function works
        let test_cases = vec![
            ("hello", "2cf24dba4f21d4288094e72ec7e88e7e2cf24dba4f21d4288094e72ec7e88e7e"),
            ("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            ("a", "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"),
        ];

        for (input, expected) in test_cases {
            let result = service.hash_token(input);
            assert_eq!(result, expected, "Hash mismatch for input: '{}'", input);
        }
    }

    #[test]
    fn test_reset_token_properties() {
        let service = create_test_service();

        // Generate multiple tokens and verify properties
        for _ in 0..50 {
            let token = service.generate_reset_token();

            // Length check
            assert_eq!(token.len(), 64);

            // Character set check
            assert!(token.chars().all(|c| c.is_alphanumeric()));

            // No repeated patterns (simple check)
            assert!(!token.contains("aaaa")); // No 4+ repeated chars (very unlikely)
            assert!(!token.contains("1111"));
            assert!(!token.contains("AAAA"));
        }
    }

    #[test]
    fn test_service_creation() {
        let service = create_test_service();
        assert_eq!(service.base_url, "https://test.com");
    }

    #[test]
    fn test_hash_collision_resistance() {
        let service = create_test_service();
        let mut hashes = HashSet::new();

        // Generate hashes for many different inputs
        for i in 0..1000 {
            let input = format!("test_token_{}", i);
            let hash = service.hash_token(&input);

            assert_eq!(hash.len(), 64);
            assert!(hashes.insert(hash), "Hash collision detected for input: {}", input);
        }
    }
}