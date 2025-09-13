use chrono::{Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::{PostgreSQL, errors::AppError};

pub struct EmailVerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used: bool,
}

pub struct EmailVerificationService {
    db: PostgreSQL,
}

impl EmailVerificationService {
    pub fn new(db: PostgreSQL) -> Self {
        Self { db }
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
