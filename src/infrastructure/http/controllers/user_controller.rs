//! HTTP User Controller with standardized error handling
//!
//! This controller translates domain AppError instances to HTTP JSON responses
//! for consistent error handling across the API.

use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands,
    errors::AppError,
    infrastructure::errors::ErrorTranslator,
    services::UserService,
    infrastructure::auth::Claims,
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
            })).into_response()
        }
        Err(app_error) => {
            error!("User creation failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
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
            ErrorTranslator::to_http_response(app_error)
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
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

#[axum::debug_handler]
pub async fn get_profile(
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    info!("Profile accessed for user: {}", claims.sub);
    Json(serde_json::json!({
        "user_id": claims.sub,
        "email": claims.email,
        "message": "This is a protected endpoint"
    })).into_response()
}