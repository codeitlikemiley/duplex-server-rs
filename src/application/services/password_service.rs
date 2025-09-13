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