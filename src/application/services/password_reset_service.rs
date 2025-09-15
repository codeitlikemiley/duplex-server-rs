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