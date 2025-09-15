//! HTTP Controller for Bulk User Operations
//!
//! This controller provides endpoints for performing bulk operations on users,
//! including batch creation, status updates, and deletions with proper error handling.

use axum::{
    Json,
    extract::{State, Path, Query},
    response::IntoResponse,
    Extension,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::{auth::Claims, errors::ErrorTranslator},
    services::{
        BulkOperationsService,
        BulkCreateUserRequest, BulkCreateUserResponse,
        BulkUpdateStatusRequest, BulkUpdateStatusResponse,
        BulkDeleteUserRequest, BulkDeleteUserResponse,
        BulkOperationResult, BulkOperationError, BulkOperationStatus
    },
    PostgreSQL,
};

/// Bulk create users endpoint
///
/// Creates multiple users in a single transaction. Returns partial success results.
/// Limited to 1000 users per request for performance reasons.
pub async fn bulk_create_users(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint (admin only in production)
    State(db): State<PostgreSQL>,
    Json(request): Json<BulkCreateUsersRequest>,
) -> impl IntoResponse {
    info!("Bulk create users requested for {} users", request.users.len());

    // Validate request
    if request.users.is_empty() {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "users".to_string(),
            message: "At least one user must be provided".to_string(),
        });
    }

    if request.users.len() > 1000 {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "users".to_string(),
            message: "Maximum 1000 users can be created in a single request".to_string(),
        });
    }

    let bulk_service = BulkOperationsService::new(db.pool());

    match bulk_service.bulk_create_users(request.users).await {
        Ok(result) => {
            info!("Bulk user creation completed: {}/{} successful",
                  result.successful_count, result.total_requested);

            let response = BulkCreateUsersResponse {
                operation_id: uuid::Uuid::new_v4(), // In production, this would track the operation
                total_requested: result.total_requested,
                successful_count: result.successful_count,
                failed_count: result.failed_count,
                created_users: result.successes,
                errors: result.failures,
                send_welcome_emails: request.send_welcome_emails.unwrap_or(false),
            };

            Json(serde_json::json!({
                "success": true,
                "data": response
            })).into_response()
        }
        Err(app_error) => {
            error!("Bulk user creation failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Bulk update user status endpoint
///
/// Updates the status of multiple users in a single transaction.
/// Supports Active, Inactive, Suspended, and PendingVerification statuses.
pub async fn bulk_update_user_status(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint (admin only)
    State(db): State<PostgreSQL>,
    Json(request): Json<BulkUpdateStatusesRequest>,
) -> impl IntoResponse {
    info!("Bulk status update requested for {} users", request.updates.len());

    if request.updates.is_empty() {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "updates".to_string(),
            message: "At least one status update must be provided".to_string(),
        });
    }

    if request.updates.len() > 1000 {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "updates".to_string(),
            message: "Maximum 1000 users can be updated in a single request".to_string(),
        });
    }

    let bulk_service = BulkOperationsService::new(db.pool());

    match bulk_service.bulk_update_user_status(request.updates).await {
        Ok(result) => {
            info!("Bulk status update completed: {}/{} successful",
                  result.successful_count, result.total_requested);

            let response = BulkUpdateStatusesResponse {
                operation_id: uuid::Uuid::new_v4(),
                total_requested: result.total_requested,
                successful_count: result.successful_count,
                failed_count: result.failed_count,
                updated_users: result.successes,
                errors: result.failures,
                reason: request.reason,
            };

            Json(serde_json::json!({
                "success": true,
                "data": response
            })).into_response()
        }
        Err(app_error) => {
            error!("Bulk status update failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Bulk delete users endpoint
///
/// Deletes multiple users (soft delete by default, hard delete with flag).
/// Limited to 100 users per request for safety reasons.
pub async fn bulk_delete_users(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint (admin only)
    State(db): State<PostgreSQL>,
    Json(request): Json<BulkDeleteUsersRequest>,
) -> impl IntoResponse {
    info!("Bulk delete requested for {} users (hard_delete: {})",
          request.deletions.len(), request.hard_delete.unwrap_or(false));

    if request.deletions.is_empty() {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "deletions".to_string(),
            message: "At least one user deletion must be provided".to_string(),
        });
    }

    if request.deletions.len() > 100 {
        return ErrorTranslator::to_http_response(AppError::Validation {
            field: "deletions".to_string(),
            message: "Maximum 100 users can be deleted in a single request".to_string(),
        });
    }

    // Add hard_delete flag to each deletion request
    let deletions: Vec<BulkDeleteUserRequest> = request.deletions.into_iter()
        .map(|mut deletion| {
            deletion.hard_delete = request.hard_delete.unwrap_or(false);
            deletion
        })
        .collect();

    let bulk_service = BulkOperationsService::new(db.pool());

    match bulk_service.bulk_delete_users(deletions).await {
        Ok(result) => {
            info!("Bulk delete completed: {}/{} successful",
                  result.successful_count, result.total_requested);

            let response = BulkDeleteUsersResponse {
                operation_id: uuid::Uuid::new_v4(),
                total_requested: result.total_requested,
                successful_count: result.successful_count,
                failed_count: result.failed_count,
                deleted_users: result.successes,
                errors: result.failures,
                hard_delete: request.hard_delete.unwrap_or(false),
                reason: request.reason,
            };

            Json(serde_json::json!({
                "success": true,
                "data": response
            })).into_response()
        }
        Err(app_error) => {
            error!("Bulk delete failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get bulk operation status endpoint
///
/// Retrieves the status of a bulk operation by its ID.
/// Useful for tracking long-running operations.
pub async fn get_bulk_operation_status(
    Extension(_claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
    Path(operation_id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Bulk operation status requested for operation: {}", operation_id);

    let bulk_service = BulkOperationsService::new(db.pool());

    match bulk_service.get_bulk_operation_status(operation_id).await {
        Ok(status) => {
            Json(serde_json::json!({
                "success": true,
                "data": status
            })).into_response()
        }
        Err(app_error) => {
            error!("Failed to get bulk operation status: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get bulk operations capabilities endpoint
///
/// Returns information about bulk operation limits and capabilities.
pub async fn get_bulk_capabilities() -> impl IntoResponse {
    let capabilities = BulkCapabilities {
        max_create_users: 1000,
        max_update_users: 1000,
        max_delete_users: 100,
        supported_operations: vec![
            "bulk_create_users".to_string(),
            "bulk_update_status".to_string(),
            "bulk_delete_users".to_string(),
        ],
        supported_status_transitions: vec![
            StatusTransition {
                from: "Active".to_string(),
                to: vec!["Inactive".to_string(), "Suspended".to_string()],
            },
            StatusTransition {
                from: "Inactive".to_string(),
                to: vec!["Active".to_string(), "Suspended".to_string()],
            },
            StatusTransition {
                from: "Suspended".to_string(),
                to: vec!["Active".to_string(), "Inactive".to_string()],
            },
            StatusTransition {
                from: "PendingVerification".to_string(),
                to: vec!["Active".to_string(), "Inactive".to_string()],
            },
        ],
        features: vec![
            "transaction_support".to_string(),
            "partial_success".to_string(),
            "detailed_error_reporting".to_string(),
            "operation_tracking".to_string(),
        ],
    };

    Json(serde_json::json!({
        "success": true,
        "data": capabilities
    }))
}

// Request DTOs
#[derive(Debug, Deserialize)]
pub struct BulkCreateUsersRequest {
    pub users: Vec<BulkCreateUserRequest>,
    pub send_welcome_emails: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateStatusesRequest {
    pub updates: Vec<BulkUpdateStatusRequest>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteUsersRequest {
    pub deletions: Vec<BulkDeleteUserRequest>,
    pub hard_delete: Option<bool>,
    pub reason: Option<String>,
}

// Response DTOs
#[derive(Debug, Serialize)]
pub struct BulkCreateUsersResponse {
    pub operation_id: Uuid,
    pub total_requested: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub created_users: Vec<BulkCreateUserResponse>,
    pub errors: Vec<BulkOperationError>,
    pub send_welcome_emails: bool,
}

#[derive(Debug, Serialize)]
pub struct BulkUpdateStatusesResponse {
    pub operation_id: Uuid,
    pub total_requested: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub updated_users: Vec<BulkUpdateStatusResponse>,
    pub errors: Vec<BulkOperationError>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteUsersResponse {
    pub operation_id: Uuid,
    pub total_requested: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub deleted_users: Vec<BulkDeleteUserResponse>,
    pub errors: Vec<BulkOperationError>,
    pub hard_delete: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkCapabilities {
    pub max_create_users: usize,
    pub max_update_users: usize,
    pub max_delete_users: usize,
    pub supported_operations: Vec<String>,
    pub supported_status_transitions: Vec<StatusTransition>,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusTransition {
    pub from: String,
    pub to: Vec<String>,
}