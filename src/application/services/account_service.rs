use chrono::Utc;
use uuid::Uuid;
use sqlx::{Pool, Postgres};

use crate::{
    errors::AppError,
    models::{User, UserStatus},
    infrastructure::repositories::PostgreSQL,
};

pub struct AccountService {
    db: Pool<Postgres>,
    postgres: PostgreSQL,
}

impl AccountService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        let postgres = PostgreSQL::new(pool.clone());
        Self { db: pool, postgres }
    }

    /// Deactivate user account (soft delete)
    pub async fn deactivate_account(
        &self,
        user_id: Uuid,
        password: &str,
    ) -> Result<(), AppError> {
        // First verify the user's password for security
        let user = self.postgres.find_user_by_id(user_id).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?
            .ok_or_else(|| AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            })?;

        // Verify password
        if crate::application::services::PasswordService::verify_password(password, &user.password_hash).is_err() {
            return Err(AppError::Authentication {
                message: "Invalid password".to_string(),
            });
        }

        // Update user status to Inactive
        sqlx::query!(
            r#"
            UPDATE users
            SET status = 'Inactive',
                updated_at = $2
            WHERE id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to deactivate account: {}", e)
        })?;

        // Invalidate all sessions
        self.invalidate_all_sessions(user_id).await?;

        // Clear profile data (optional - depends on requirements)
        self.clear_profile_data(user_id).await?;

        tracing::info!("Account deactivated for user: {}", user_id);
        Ok(())
    }

    /// Reactivate user account
    pub async fn reactivate_account(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, AppError> {
        // Find user by email
        let mut user = self.postgres.find_user_by_email(email).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?
            .ok_or_else(|| AppError::NotFound {
                resource: "User".to_string(),
                id: None,
            })?;

        // Verify password
        if crate::application::services::PasswordService::verify_password(password, &user.password_hash).is_err() {
            return Err(AppError::Authentication {
                message: "Invalid password".to_string(),
            });
        }

        // Check if account is actually inactive
        if user.status != UserStatus::Inactive {
            return Err(AppError::Validation {
                field: "status".to_string(),
                message: "Account is not deactivated".to_string(),
            });
        }

        // Reactivate account
        user.status = UserStatus::Active;
        user.updated_at = Utc::now();

        sqlx::query!(
            r#"
            UPDATE users
            SET status = 'Active',
                updated_at = $2
            WHERE id = $1
            "#,
            user.id,
            user.updated_at
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to reactivate account: {}", e)
        })?;

        tracing::info!("Account reactivated for user: {}", user.id);
        Ok(user)
    }

    /// Permanently delete user account (hard delete)
    pub async fn delete_account(
        &self,
        user_id: Uuid,
        password: &str,
        confirmation: &str,
    ) -> Result<(), AppError> {
        // Verify confirmation phrase
        if confirmation != "DELETE MY ACCOUNT" {
            return Err(AppError::Validation {
                field: "confirmation".to_string(),
                message: "Please type 'DELETE MY ACCOUNT' to confirm".to_string(),
            });
        }

        // Verify the user's password for security
        let user = self.postgres.find_user_by_id(user_id).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?
            .ok_or_else(|| AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            })?;

        // Verify password
        if crate::application::services::PasswordService::verify_password(password, &user.password_hash).is_err() {
            return Err(AppError::Authentication {
                message: "Invalid password".to_string(),
            });
        }

        // Start transaction for data consistency
        let mut tx = self.db.begin().await
            .map_err(|e| AppError::Database {
                message: format!("Failed to start transaction: {}", e)
            })?;

        // Delete related data in order (respecting foreign key constraints)

        // Delete email verification tokens
        sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE user_id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete verification tokens: {}", e)
        })?;

        // Delete password reset tokens
        sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete reset tokens: {}", e)
        })?;

        // Delete user sessions
        sqlx::query!(
            "DELETE FROM user_sessions WHERE user_id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete sessions: {}", e)
        })?;

        // Delete user profile
        sqlx::query!(
            "DELETE FROM user_profiles WHERE user_id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete profile: {}", e)
        })?;

        // Finally, delete the user
        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete user: {}", e)
        })?;

        // Commit transaction
        tx.commit().await
            .map_err(|e| AppError::Database {
                message: format!("Failed to commit transaction: {}", e)
            })?;

        tracing::info!("Account permanently deleted for user: {}", user_id);
        Ok(())
    }

    /// Suspend user account (admin action)
    pub async fn suspend_account(
        &self,
        user_id: Uuid,
        reason: &str,
        admin_id: Uuid,
    ) -> Result<(), AppError> {
        // Update user status to Suspended
        sqlx::query!(
            r#"
            UPDATE users
            SET status = 'Suspended',
                updated_at = $2
            WHERE id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to suspend account: {}", e)
        })?;

        // Log the suspension action (audit trail)
        // This would typically go to an audit log table
        tracing::warn!(
            "Account suspended - User: {}, Reason: {}, Admin: {}",
            user_id,
            reason,
            admin_id
        );

        // Invalidate all sessions
        self.invalidate_all_sessions(user_id).await?;

        Ok(())
    }

    /// Helper: Invalidate all user sessions
    async fn invalidate_all_sessions(&self, user_id: Uuid) -> Result<(), AppError> {
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

    /// Helper: Clear profile data (soft delete)
    async fn clear_profile_data(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE user_profiles
            SET first_name = NULL,
                last_name = NULL,
                bio = NULL,
                avatar_url = NULL,
                website = NULL,
                location = NULL,
                updated_at = $2
            WHERE user_id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to clear profile: {}", e)
        })?;

        Ok(())
    }

    /// Get deactivated accounts (admin function)
    pub async fn list_deactivated_accounts(&self, limit: i64, offset: i64) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, email_verified,
                   status as "status: UserStatus",
                   created_at, updated_at, last_login_at
            FROM users
            WHERE status = 'Inactive'
            ORDER BY updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to list deactivated accounts: {}", e)
        })?;

        Ok(users)
    }
}

// Request DTOs
#[derive(Debug, serde::Deserialize)]
pub struct DeactivateAccountRequest {
    pub password: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ReactivateAccountRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeleteAccountRequest {
    pub password: String,
    pub confirmation: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct SuspendAccountRequest {
    pub user_id: Uuid,
    pub reason: String,
}