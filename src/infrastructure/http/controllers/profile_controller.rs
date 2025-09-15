use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{ProfileService, UpdateProfileRequest, UpdateAccountRequest},
};

/// Get user profile
pub async fn get_user_profile(
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

    let profile_service = ProfileService::new(db.pool());

    match profile_service.get_profile(user_id).await {
        Ok(Some(profile)) => {
            info!("Profile retrieved for user: {}", user_id);
            Json(serde_json::json!({
                "profile": profile
            }))
            .into_response()
        }
        Ok(None) => {
            info!("No profile found for user: {}", user_id);
            Json(serde_json::json!({
                "profile": null,
                "message": "No profile exists yet"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get profile for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Update user profile
pub async fn update_user_profile(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let profile_service = ProfileService::new(db.pool());

    match profile_service.update_profile(user_id, payload).await {
        Ok(profile) => {
            info!("Profile updated for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Profile updated successfully",
                "profile": profile
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to update profile for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Update user account (username, email)
pub async fn update_user_account(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<UpdateAccountRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let profile_service = ProfileService::new(db.pool());

    match profile_service.update_account(user_id, payload).await {
        Ok(user) => {
            info!("Account updated for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Account updated successfully",
                "user": {
                    "id": user.id,
                    "username": user.username,
                    "email": user.email,
                    "email_verified": user.email_verified
                },
                "requires_verification": !user.email_verified
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to update account for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Delete user profile (soft delete)
pub async fn delete_user_profile(
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

    let profile_service = ProfileService::new(db.pool());

    match profile_service.delete_profile(user_id).await {
        Ok(()) => {
            info!("Profile deleted for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Profile deleted successfully"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to delete profile for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}