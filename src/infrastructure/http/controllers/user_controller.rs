//! HTTP User Controller with standardized error handling
//!
//! This controller translates domain AppError instances to HTTP JSON responses
//! for consistent error handling across the API.

use axum::{
    Json,
    extract::{Extension, Path, State},
    response::IntoResponse,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands, errors::AppError, infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator, services::{UserService, PasswordService},
};

pub async fn create_user(
    State(handler): State<UserService>,
    Json(payload): Json<commands::CreateUser>,
) -> impl IntoResponse {
    match handler.handle_create_user(payload).await {
        Ok(()) => {
            info!("User creation successful");
            Json(serde_json::json!({
                "message": "User creation initiated successfully"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("User creation failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error.into())
        }
    }
}

pub async fn get_user_by_id(
    State(state): State<UserService>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.handle_get_user_by_id(id).await {
        Ok(Some(user)) => {
            info!("User Found: {} ({})", user.username, user.email);
            Json(user).into_response()
        }
        Ok(None) => {
            let error = AppError::NotFound {
                resource: "User".to_string(),
                id: Some(id.to_string()),
            };
            info!("User Not Found: {}", id);
            ErrorTranslator::to_http_response(error)
        }
        Err(app_error) => {
            error!("Failed to fetch user {}: {:?}", id, app_error);
            ErrorTranslator::to_http_response(app_error.into())
        }
    }
}

pub async fn login(
    State(state): State<UserService>,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    let email = payload.email.clone(); // Clone email before moving payload
    match state.handle_login(payload).await {
        Ok(token) => {
            info!("Login successful for user: {}", email);
            Json(serde_json::json!({"token": token})).into_response()
        }
        Err(app_error) => {
            error!("Login failed for user {}: {:?}", email, app_error);
            ErrorTranslator::to_http_response(app_error.into())
        }
    }
}

pub async fn register_user(
    State(handler): State<UserService>,
    Json(payload): Json<commands::RegisterUser>,
) -> impl IntoResponse {
    let email = payload.email.clone();
    match handler.handle_register_user(payload).await {
        Ok(()) => {
            info!("User registration successful for: {}", email);
            Json(serde_json::json!({
                "message": "Registration successful. Please check your email for verification."
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Registration failed for {}: {:?}", email, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

pub async fn verify_email(
    State(handler): State<UserService>,
    Json(payload): Json<commands::VerifyEmail>,
) -> impl IntoResponse {
    match handler.handle_verify_email(payload).await {
        Ok(()) => {
            info!("Email verification successful");
            Json(serde_json::json!({
                "message": "Email verified successfully. You can now login."
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Email verification failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Json(payload): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let password_service = PasswordService::new(db);
    match password_service
        .change_password(user_id, &payload.current_password, &payload.new_password)
        .await
    {
        Ok(()) => {
            info!("Password changed successfully for user: {}", user_id);
            Json(serde_json::json!({
                "message": "Password changed successfully"
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Password change failed for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

#[axum::debug_handler]
pub async fn get_profile(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    info!("Profile accessed for user: {}", claims.sub);
    Json(serde_json::json!({
        "user_id": claims.sub,
        "email": claims.email,
        "message": "This is a protected endpoint"
    }))
    .into_response()
}
