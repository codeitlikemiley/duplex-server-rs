use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{AccountService, DeactivateAccountRequest, ReactivateAccountRequest, DeleteAccountRequest},
};

/// Deactivate user account (soft delete)
pub async fn deactivate_account(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<DeactivateAccountRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let account_service = AccountService::new(db.pool());

    match account_service.deactivate_account(user_id, &payload.password).await {
        Ok(()) => {
            warn!("Account deactivated for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Account deactivated successfully. You can reactivate it by logging in again.",
                "status": "deactivated"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to deactivate account for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Reactivate user account
pub async fn reactivate_account(
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<ReactivateAccountRequest>,
) -> impl IntoResponse {
    let account_service = AccountService::new(db.pool());

    match account_service.reactivate_account(&payload.email, &payload.password).await {
        Ok(user) => {
            info!("Account reactivated for user: {}", user.id);

            // Generate new login token
            let session_service = crate::services::SessionService::new(crate::infrastructure::repositories::PostgreSQL::new(db.pool()));
            match session_service.create_session(
                user.id,
                &user.email,
                Some("0.0.0.0".to_string()),
                Some("Reactivation".to_string())
            ).await {
                Ok(token) => {
                    Json(serde_json::json!({
                        "message": "Account reactivated successfully",
                        "token": token,
                        "user": {
                            "id": user.id,
                            "username": user.username,
                            "email": user.email
                        }
                    }))
                    .into_response()
                }
                Err(_) => {
                    Json(serde_json::json!({
                        "message": "Account reactivated successfully. Please login again.",
                        "user": {
                            "id": user.id,
                            "username": user.username,
                            "email": user.email
                        }
                    }))
                    .into_response()
                }
            }
        }
        Err(app_error) => {
            error!("Failed to reactivate account: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Permanently delete user account (hard delete)
pub async fn delete_account(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<DeleteAccountRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let account_service = AccountService::new(db.pool());

    match account_service.delete_account(user_id, &payload.password, &payload.confirmation).await {
        Ok(()) => {
            warn!("Account permanently deleted for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Your account has been permanently deleted. We're sorry to see you go.",
                "status": "deleted"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to delete account for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get account status
pub async fn get_account_status(
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

    match db.find_user_by_id(user_id).await {
        Ok(Some(user)) => {
            Json(serde_json::json!({
                "user_id": user.id,
                "username": user.username,
                "email": user.email,
                "status": user.status,
                "email_verified": user.email_verified,
                "created_at": user.created_at,
                "last_login_at": user.last_login_at
            }))
            .into_response()
        }
        Ok(None) => {
            ErrorTranslator::to_http_response(AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            })
        }
        Err(e) => {
            error!("Failed to get account status: {:?}", e);
            ErrorTranslator::to_http_response(AppError::Database {
                message: "Failed to retrieve account status".to_string()
            })
        }
    }
}