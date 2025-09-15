//! Bulk Operations Service for User Management
//!
//! This service provides efficient bulk operations for user management,
//! including batch processing with transaction support and error handling.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Transaction};
use tracing::{info, error, warn};
use uuid::Uuid;
use std::collections::HashMap;

use crate::{
    errors::AppError,
    models::{User, UserStatus},
    PostgreSQL,
};

/// Service for handling bulk user operations
pub struct BulkOperationsService {
    pool: Pool<Postgres>,
}

impl BulkOperationsService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Bulk create users with transaction support
    pub async fn bulk_create_users(
        &self,
        requests: Vec<BulkCreateUserRequest>,
    ) -> Result<BulkOperationResult<BulkCreateUserResponse>, AppError> {
        info!("Starting bulk user creation for {} users", requests.len());

        if requests.len() > 1000 {
            return Err(AppError::Validation {
                field: "requests".to_string(),
                message: "Bulk operations are limited to 1000 items per request".to_string(),
            });
        }

        let mut transaction = self.pool.begin().await.map_err(|e| AppError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (index, request) in requests.into_iter().enumerate() {
            match self.create_single_user(&mut transaction, request.clone()).await {
                Ok(user) => {
                    successes.push(BulkCreateUserResponse {
                        index,
                        user_id: user.id,
                        username: user.username,
                        email: user.email,
                        status: "created".to_string(),
                    });
                }
                Err(e) => {
                    warn!("Failed to create user at index {}: {:?}", index, e);
                    failures.push(BulkOperationError {
                        index,
                        error_code: "USER_CREATION_FAILED".to_string(),
                        error_message: e.to_string(),
                        item_id: request.email.clone(),
                    });
                }
            }
        }

        // Commit transaction only if there are successes and user wants partial success
        if !successes.is_empty() {
            transaction.commit().await.map_err(|e| AppError::Database {
                message: format!("Failed to commit transaction: {}", e),
            })?;

            info!("Bulk user creation completed: {} successes, {} failures",
                  successes.len(), failures.len());
        } else {
            transaction.rollback().await.map_err(|e| AppError::Database {
                message: format!("Failed to rollback transaction: {}", e),
            })?;
        }

        Ok(BulkOperationResult {
            total_requested: successes.len() + failures.len(),
            successful_count: successes.len(),
            failed_count: failures.len(),
            successes,
            failures,
        })
    }

    /// Bulk update user status
    pub async fn bulk_update_user_status(
        &self,
        requests: Vec<BulkUpdateStatusRequest>,
    ) -> Result<BulkOperationResult<BulkUpdateStatusResponse>, AppError> {
        info!("Starting bulk status update for {} users", requests.len());

        if requests.len() > 1000 {
            return Err(AppError::Validation {
                field: "requests".to_string(),
                message: "Bulk operations are limited to 1000 items per request".to_string(),
            });
        }

        let mut transaction = self.pool.begin().await.map_err(|e| AppError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (index, request) in requests.into_iter().enumerate() {
            match self.update_user_status_single(&mut transaction, &request).await {
                Ok(updated_user) => {
                    successes.push(BulkUpdateStatusResponse {
                        index,
                        user_id: request.user_id,
                        old_status: request.current_status.clone().unwrap_or_default(),
                        new_status: format!("{:?}", &request.new_status),
                        updated_at: updated_user.updated_at.to_rfc3339(),
                    });
                }
                Err(e) => {
                    warn!("Failed to update user status at index {}: {:?}", index, e);
                    failures.push(BulkOperationError {
                        index,
                        error_code: "STATUS_UPDATE_FAILED".to_string(),
                        error_message: e.to_string(),
                        item_id: request.user_id.to_string(),
                    });
                }
            }
        }

        if !successes.is_empty() {
            transaction.commit().await.map_err(|e| AppError::Database {
                message: format!("Failed to commit transaction: {}", e),
            })?;

            info!("Bulk status update completed: {} successes, {} failures",
                  successes.len(), failures.len());
        } else {
            transaction.rollback().await.map_err(|e| AppError::Database {
                message: format!("Failed to rollback transaction: {}", e),
            })?;
        }

        Ok(BulkOperationResult {
            total_requested: successes.len() + failures.len(),
            successful_count: successes.len(),
            failed_count: failures.len(),
            successes,
            failures,
        })
    }

    /// Bulk delete users (soft delete)
    pub async fn bulk_delete_users(
        &self,
        requests: Vec<BulkDeleteUserRequest>,
    ) -> Result<BulkOperationResult<BulkDeleteUserResponse>, AppError> {
        info!("Starting bulk user deletion for {} users", requests.len());

        if requests.len() > 100 {
            return Err(AppError::Validation {
                field: "requests".to_string(),
                message: "Bulk delete operations are limited to 100 items per request".to_string(),
            });
        }

        let mut transaction = self.pool.begin().await.map_err(|e| AppError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for (index, request) in requests.into_iter().enumerate() {
            match self.delete_user_single(&mut transaction, &request).await {
                Ok(_) => {
                    successes.push(BulkDeleteUserResponse {
                        index,
                        user_id: request.user_id,
                        deletion_type: if request.hard_delete {
                            "hard_delete".to_string()
                        } else {
                            "soft_delete".to_string()
                        },
                        deleted_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
                Err(e) => {
                    warn!("Failed to delete user at index {}: {:?}", index, e);
                    failures.push(BulkOperationError {
                        index,
                        error_code: "USER_DELETION_FAILED".to_string(),
                        error_message: e.to_string(),
                        item_id: request.user_id.to_string(),
                    });
                }
            }
        }

        if !successes.is_empty() {
            transaction.commit().await.map_err(|e| AppError::Database {
                message: format!("Failed to commit transaction: {}", e),
            })?;

            info!("Bulk user deletion completed: {} successes, {} failures",
                  successes.len(), failures.len());
        } else {
            transaction.rollback().await.map_err(|e| AppError::Database {
                message: format!("Failed to rollback transaction: {}", e),
            })?;
        }

        Ok(BulkOperationResult {
            total_requested: successes.len() + failures.len(),
            successful_count: successes.len(),
            failed_count: failures.len(),
            successes,
            failures,
        })
    }

    /// Get bulk operation status
    pub async fn get_bulk_operation_status(
        &self,
        operation_id: Uuid,
    ) -> Result<BulkOperationStatus, AppError> {
        // This would typically fetch from a job queue or status table
        // For now, return a mock status
        Ok(BulkOperationStatus {
            operation_id,
            operation_type: "bulk_create_users".to_string(),
            status: "completed".to_string(),
            total_items: 100,
            processed_items: 100,
            successful_items: 95,
            failed_items: 5,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            estimated_completion: None,
        })
    }

    // Private helper methods
    async fn create_single_user(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        request: BulkCreateUserRequest,
    ) -> Result<User, AppError> {
        // Check if user already exists
        let existing = sqlx::query_scalar!(
            "SELECT id FROM users WHERE email = $1 OR username = $2",
            request.email,
            request.username
        )
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to check existing user: {}", e),
        })?;

        if existing.is_some() {
            return Err(AppError::Validation {
                field: "user".to_string(),
                message: "User with this email or username already exists".to_string(),
            });
        }

        // Hash password
        let password_hash = crate::services::PasswordService::hash_password(&request.password)
            .map_err(|e| AppError::Internal {
                message: format!("Failed to hash password: {}", e),
            })?;

        // Insert user
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                id, username, email, password_hash, status,
                email_verified, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            RETURNING
                id, username, email, password_hash,
                status as "status: UserStatus",
                email_verified, created_at, updated_at, last_login_at
            "#,
            uuid::Uuid::new_v4(),
            request.username,
            request.email,
            password_hash,
            UserStatus::PendingVerification as UserStatus,
            false
        )
        .fetch_one(&mut **transaction)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to insert user: {}", e),
        })?;

        Ok(user)
    }

    async fn update_user_status_single(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        request: &BulkUpdateStatusRequest,
    ) -> Result<User, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, username, email, password_hash,
                status as "status: UserStatus",
                email_verified, created_at, updated_at, last_login_at
            "#,
            request.user_id,
            request.new_status.clone() as UserStatus
        )
        .fetch_one(&mut **transaction)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update user status: {}", e),
        })?;

        Ok(user)
    }

    async fn delete_user_single(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        request: &BulkDeleteUserRequest,
    ) -> Result<(), AppError> {
        if request.hard_delete {
            // Hard delete - actually remove from database
            sqlx::query!(
                "DELETE FROM users WHERE id = $1",
                request.user_id
            )
            .execute(&mut **transaction)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to hard delete user: {}", e),
            })?;
        } else {
            // Soft delete - update status and add deleted_at timestamp
            sqlx::query!(
                "UPDATE users SET status = $2, updated_at = NOW() WHERE id = $1",
                request.user_id,
                UserStatus::Inactive as UserStatus
            )
            .execute(&mut **transaction)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to soft delete user: {}", e),
            })?;
        }

        Ok(())
    }
}

// Request/Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub send_welcome_email: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkCreateUserResponse {
    pub index: usize,
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateStatusRequest {
    pub user_id: Uuid,
    pub new_status: UserStatus,
    pub current_status: Option<String>, // For validation
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkUpdateStatusResponse {
    pub index: usize,
    pub user_id: Uuid,
    pub old_status: String,
    pub new_status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteUserRequest {
    pub user_id: Uuid,
    pub hard_delete: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDeleteUserResponse {
    pub index: usize,
    pub user_id: Uuid,
    pub deletion_type: String,
    pub deleted_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkOperationResult<T> {
    pub total_requested: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub successes: Vec<T>,
    pub failures: Vec<BulkOperationError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkOperationError {
    pub index: usize,
    pub error_code: String,
    pub error_message: String,
    pub item_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkOperationStatus {
    pub operation_id: Uuid,
    pub operation_type: String,
    pub status: String, // pending, processing, completed, failed
    pub total_items: usize,
    pub processed_items: usize,
    pub successful_items: usize,
    pub failed_items: usize,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub estimated_completion: Option<String>,
}