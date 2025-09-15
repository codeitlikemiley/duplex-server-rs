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
    services::{UserService, RbacService, ProfileService},
    models::UserStatus,
};

/// V2 User representation (enhanced with more fields)
#[derive(Debug, Serialize)]
pub struct UserV2 {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub roles: Vec<String>, // V2 includes user roles
}

/// V2 Registration request (includes name fields)
#[derive(Debug, Deserialize)]
pub struct RegisterV2Request {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// V2 Profile representation (enhanced)
#[derive(Debug, Serialize)]
pub struct ProfileV2 {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// V2 Login response (enhanced with user info)
#[derive(Debug, Serialize)]
pub struct LoginV2Response {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64, // seconds
    pub user: UserBasicInfo,
}

#[derive(Debug, Serialize)]
pub struct UserBasicInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
}

/// Get user profile (V2 - enhanced response)
pub async fn get_profile_v2(
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
                    let profile_v2 = ProfileV2 {
                        user_id: user.id,
                        first_name: profile.first_name,
                        last_name: profile.last_name,
                        email: user.email,
                        bio: profile.bio,
                        location: profile.location,
                        website: profile.website,
                        avatar_url: profile.avatar_url,
                        created_at: profile.created_at,
                        updated_at: profile.updated_at,
                    };
                    versioned_json(profile_v2, ApiVersion::V2).into_response()
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

/// Register user (V2 - enhanced registration)
pub async fn register_v2(
    State(user_service): State<UserService>,
    Json(payload): Json<RegisterV2Request>,
) -> impl IntoResponse {
    let cmd = commands::RegisterUser {
        username: payload.username,
        email: payload.email,
        password: payload.password,
        first_name: payload.first_name,
        last_name: payload.last_name,
    };

    match user_service.handle_register_user(cmd).await {
        Ok(()) => {
            let response = serde_json::json!({
                "success": true,
                "message": "User registered successfully",
                "next_steps": {
                    "email_verification": "Check your email for verification link",
                    "login": "You can login after email verification"
                }
            });
            versioned_json(response, ApiVersion::V2).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}

/// Get user by ID (V2 - enhanced response with roles)
pub async fn get_user_v2(
    Path(user_id): Path<Uuid>,
    State(user_service): State<UserService>,
) -> impl IntoResponse {
    match user_service.repo.find_user_by_id(user_id).await {
        Ok(Some(user)) => {
            // Get user roles
            let rbac_service = RbacService::new(user_service.repo.db.clone());
            let roles = match rbac_service.get_user_roles(user_id).await {
                Ok(user_roles) => user_roles.into_iter().map(|r| r.name).collect(),
                Err(_) => vec![], // Don't fail the request if roles can't be fetched
            };

            let user_v2 = UserV2 {
                id: user.id,
                username: user.username,
                email: user.email,
                email_verified: user.email_verified,
                status: user.status,
                created_at: user.created_at,
                updated_at: user.updated_at,
                last_login_at: user.last_login_at,
                roles,
            };
            versioned_json(user_v2, ApiVersion::V2).into_response()
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

/// Login (V2 - enhanced response with user info)
pub async fn login_v2(
    State(user_service): State<UserService>,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    match user_service.handle_login(payload.clone()).await {
        Ok(token) => {
            // Get user info for enhanced response
            let user_info = match user_service.repo.find_user_by_email(&payload.email).await {
                Ok(Some(user)) => {
                    // Get user roles
                    let rbac_service = RbacService::new(user_service.repo.db.clone());
                    let roles = match rbac_service.get_user_roles(user.id).await {
                        Ok(user_roles) => user_roles.into_iter().map(|r| r.name).collect(),
                        Err(_) => vec![],
                    };

                    UserBasicInfo {
                        id: user.id,
                        username: user.username,
                        email: user.email,
                        roles,
                    }
                }
                _ => {
                    // If we can't get user info, return error
                    return ErrorTranslator::to_http_response(crate::errors::AppError::Internal {
                        message: "Failed to retrieve user information".to_string(),
                    });
                }
            };

            let response = LoginV2Response {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: 7 * 24 * 60 * 60, // 7 days in seconds
                user: user_info,
            };

            versioned_json(response, ApiVersion::V2).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}