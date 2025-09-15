use axum::{
    Json,
    extract::{Extension, State, Query, Path},
    response::IntoResponse,
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{AccountLockoutService, ManualLockoutRequest, UnlockAccountRequest},
};

/// Get lockout status for the current user
pub async fn get_lockout_status(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let lockout_service = AccountLockoutService::new(db.pool());

    match lockout_service.is_account_locked(user_id).await {
        Ok(lockout_status) => {
            Json(serde_json::json!({
                "success": true,
                "lockout_status": lockout_status
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get lockout status for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get lockout history for the current user
pub async fn get_user_lockout_history(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<HistoryQuery>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let lockout_service = AccountLockoutService::new(db.pool());
    let limit = params.limit.unwrap_or(10).min(50); // Max 50 records

    match lockout_service.get_lockout_history(user_id, limit).await {
        Ok(history) => {
            info!("Lockout history retrieved for user: {} (limit: {})", user_id, limit);
            Json(serde_json::json!({
                "success": true,
                "history": history,
                "limit": limit
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get lockout history for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Manually lock a user account (admin feature)
pub async fn lock_user_account(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Path(target_user_id): Path<Uuid>,
    Json(payload): Json<ManualLockoutRequest>,
) -> impl IntoResponse {
    let admin_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let lockout_service = AccountLockoutService::new(db.pool());

    match lockout_service.manually_lock_account(
        target_user_id,
        admin_id,
        payload.lockout_type,
        payload.reason,
        payload.duration_hours,
    ).await {
        Ok(lockout_info) => {
            warn!("User account {} manually locked by admin {}: {:?}", target_user_id, admin_id, lockout_info);
            Json(serde_json::json!({
                "success": true,
                "message": "User account has been locked",
                "lockout_info": lockout_info
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to lock user account {} by admin {}: {:?}", target_user_id, admin_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Unlock a user account (admin feature)
pub async fn unlock_user_account(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Path(target_user_id): Path<Uuid>,
    Json(payload): Json<UnlockAccountRequest>,
) -> impl IntoResponse {
    let admin_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let lockout_service = AccountLockoutService::new(db.pool());

    match lockout_service.unlock_account(target_user_id, admin_id, payload.reason).await {
        Ok(()) => {
            info!("User account {} unlocked by admin {}", target_user_id, admin_id);
            Json(serde_json::json!({
                "success": true,
                "message": "User account has been unlocked",
                "user_id": target_user_id
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to unlock user account {} by admin {}: {:?}", target_user_id, admin_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get lockout statistics (admin feature)
pub async fn get_lockout_statistics(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
) -> impl IntoResponse {
    let admin_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let lockout_service = AccountLockoutService::new(db.pool());

    match lockout_service.get_lockout_stats().await {
        Ok(stats) => {
            info!("Lockout statistics retrieved by admin: {}", admin_id);
            Json(serde_json::json!({
                "success": true,
                "statistics": stats
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get lockout statistics: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get lockout history for a specific user (admin feature)
pub async fn get_admin_lockout_history(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Path(target_user_id): Path<Uuid>,
    Query(params): Query<HistoryQuery>,
) -> impl IntoResponse {
    let admin_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let lockout_service = AccountLockoutService::new(db.pool());
    let limit = params.limit.unwrap_or(10).min(100); // Max 100 records for admin

    match lockout_service.get_lockout_history(target_user_id, limit).await {
        Ok(history) => {
            info!("Lockout history for user {} retrieved by admin: {} (limit: {})", target_user_id, admin_id, limit);
            Json(serde_json::json!({
                "success": true,
                "user_id": target_user_id,
                "history": history,
                "limit": limit
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get lockout history for user {} by admin {}: {:?}", target_user_id, admin_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Cleanup expired lockouts (admin maintenance feature)
pub async fn cleanup_expired_lockouts(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
) -> impl IntoResponse {
    let admin_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let lockout_service = AccountLockoutService::new(db.pool());

    match lockout_service.cleanup_expired_lockouts().await {
        Ok(cleaned_count) => {
            info!("Cleaned up {} expired lockouts by admin: {}", cleaned_count, admin_id);
            Json(serde_json::json!({
                "success": true,
                "message": format!("Cleaned up {} expired lockouts", cleaned_count),
                "cleaned_count": cleaned_count
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to cleanup expired lockouts: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

// Query parameter DTOs
#[derive(Debug, serde::Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i32>,
}