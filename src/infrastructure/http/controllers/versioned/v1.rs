use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    commands,
    infrastructure::{
        errors::ErrorTranslator,
        auth::Claims,
        http::versioning::{ApiVersion, versioned_json},
    },
    services::{UserService, ProfileService},
};

/// V1 User representation (limited fields for backward compatibility)
#[derive(Debug, Serialize)]
pub struct UserV1 {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: String, // Simplified status representation
}

/// V1 Registration request (basic fields only)
#[derive(Debug, Deserialize)]
pub struct RegisterV1Request {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// V1 Profile representation (basic fields)
#[derive(Debug, Serialize)]
pub struct ProfileV1 {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
}

/// Get user profile (V1 - limited response)
pub async fn get_profile_v1(
    Extension(claims): Extension<Claims>,
    State(user_service): State<UserService>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(crate::errors::AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    match user_service.repo.find_user_by_id(user_id).await {
        Ok(Some(user)) => {
            let profile_service = ProfileService::new(user_service.repo.pool());
            match profile_service.get_profile(user_id).await {
                Ok(Some(profile)) => {
                    let profile_v1 = ProfileV1 {
                        user_id: user.id,
                        first_name: profile.first_name,
                        last_name: profile.last_name,
                        email: user.email,
                    };
                    versioned_json(profile_v1, ApiVersion::V1).into_response()
                }
                Ok(None) => {
                    ErrorTranslator::to_http_response(crate::errors::AppError::NotFound {
                        resource: "Profile".to_string(),
                        id: Some(user_id.to_string()),
                    })
                }
                Err(e) => ErrorTranslator::to_http_response(e.into()),
            }
        }
        Ok(None) => {
            ErrorTranslator::to_http_response(crate::errors::AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            })
        }
        Err(e) => ErrorTranslator::to_http_response(e.into()),
    }
}

/// Register user (V1 - basic registration)
pub async fn register_v1(
    State(user_service): State<UserService>,
    Json(payload): Json<RegisterV1Request>,
) -> impl IntoResponse {
    // Convert V1 request to internal command
    let cmd = commands::RegisterUser {
        username: payload.username,
        email: payload.email,
        password: payload.password,
        first_name: None, // V1 doesn't support names during registration
        last_name: None,
    };

    match user_service.handle_register_user(cmd).await {
        Ok(()) => {
            let response = serde_json::json!({
                "message": "User registered successfully",
                "note": "Email verification required"
            });
            versioned_json(response, ApiVersion::V1).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}

/// Get user by ID (V1 - limited response)
pub async fn get_user_v1(
    Path(user_id): Path<Uuid>,
    State(user_service): State<UserService>,
) -> impl IntoResponse {
    match user_service.repo.find_user_by_id(user_id).await {
        Ok(Some(user)) => {
            let user_v1 = UserV1 {
                id: user.id,
                username: user.username,
                email: user.email,
                created_at: user.created_at,
                status: match user.status {
                    crate::models::UserStatus::Active => "active".to_string(),
                    crate::models::UserStatus::PendingVerification => "pending".to_string(),
                    crate::models::UserStatus::Suspended => "suspended".to_string(),
                    crate::models::UserStatus::Inactive => "inactive".to_string(),
                },
            };
            versioned_json(user_v1, ApiVersion::V1).into_response()
        }
        Ok(None) => {
            ErrorTranslator::to_http_response(crate::errors::AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            })
        }
        Err(e) => ErrorTranslator::to_http_response(e.into()),
    }
}

/// Login (V1 - basic response)
pub async fn login_v1(
    State(user_service): State<UserService>,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    match user_service.handle_login(payload).await {
        Ok(token) => {
            let response = serde_json::json!({
                "token": token,
                "token_type": "Bearer"
            });
            versioned_json(response, ApiVersion::V1).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}