use std::sync::Arc;
use chrono::{Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    errors::AppError,
    infrastructure::email::{EmailService, EmailTemplates},
};

pub struct EmailVerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used: bool,
}

pub struct EmailVerificationService {
    db: PostgreSQL,
    email_service: Arc<dyn EmailService>,
    base_url: String,
}

impl EmailVerificationService {
    pub fn new(db: PostgreSQL, email_service: Arc<dyn EmailService>) -> Self {
        let base_url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:80".to_string());

        Self {
            db,
            email_service,
            base_url,
        }
    }

    /// Generate a secure random verification token
    pub fn generate_token() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        let mut rng = rand::rng();
        (0..64)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Create and store a verification token for a user
    pub async fn create_verification_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let token = Self::generate_token();
        let expires_at = Utc::now() + Duration::hours(24); // Token expires in 24 hours

        sqlx::query!(
            r#"
            INSERT INTO email_verification_tokens (id, user_id, token, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::now_v7(),
            user_id,
            token,
            expires_at
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create verification token: {}", e),
        })?;

        Ok(token)
    }

    /// Send verification email to user
    pub async fn send_verification_email(
        &self,
        user_id: Uuid,
        email: &str,
        username: &str,
    ) -> Result<(), AppError> {
        // Create verification token
        let token = self.create_verification_token(user_id).await?;

        // Create verification URL
        let verification_url = format!(
            "{}/verify-email?user_id={}&token={}",
            self.base_url, user_id, token
        );

        // Create email from template
        let from_email = std::env::var("FROM_EMAIL")
            .unwrap_or_else(|_| "noreply@quake.app".to_string());

        let email_message = EmailTemplates::verification_email(
            email,
            &from_email,
            username,
            &verification_url,
        );

        // Send email
        self.email_service.send_email(email_message).await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to send verification email: {}", e),
            })?;

        tracing::info!(
            "Verification email sent to {} for user {}",
            email, user_id
        );

        Ok(())
    }

    /// Verify an email token and activate the user account
    pub async fn verify_email_token(&self, user_id: Uuid, token: &str) -> Result<(), AppError> {
        // Check if token exists and is valid
        let token_record = sqlx::query!(
            r#"
            SELECT id, expires_at, used
            FROM email_verification_tokens
            WHERE user_id = $1 AND token = $2
            "#,
            user_id,
            token
        )
        .fetch_optional(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch verification token: {}", e),
        })?;

        let token_record = token_record.ok_or_else(|| AppError::Validation {
            field: "token".to_string(),
            message: "Invalid or expired verification token".to_string(),
        })?;

        // Check if token is already used
        if token_record.used {
            return Err(AppError::Validation {
                field: "token".to_string(),
                message: "Verification token has already been used".to_string(),
            });
        }

        // Check if token is expired
        if token_record.expires_at < Utc::now() {
            return Err(AppError::Validation {
                field: "token".to_string(),
                message: "Verification token has expired".to_string(),
            });
        }

        // Start a transaction to update both token and user status
        let mut tx = self.db.db.begin().await.map_err(|e| AppError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Mark token as used
        sqlx::query!(
            r#"
            UPDATE email_verification_tokens
            SET used = true
            WHERE id = $1
            "#,
            token_record.id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to mark token as used: {}", e),
        })?;

        // Update user status to Active and mark email as verified
        sqlx::query!(
            r#"
            UPDATE users
            SET email_verified = true,
                status = 'Active',
                updated_at = $2
            WHERE id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update user status: {}", e),
        })?;

        // Commit the transaction
        tx.commit().await.map_err(|e| AppError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        tracing::info!("Email verified successfully for user: {}", user_id);
        Ok(())
    }

    /// Clean up expired tokens (can be run periodically)
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM email_verification_tokens
            WHERE expires_at < $1 OR used = true
            "#,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup expired tokens: {}", e),
        })?;

        Ok(result.rows_affected())
    }
}
