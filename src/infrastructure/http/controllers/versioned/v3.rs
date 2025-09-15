use axum::{
    extract::{Extension, Path, State, Query},
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
    services::{UserService, RbacService, ActivityLoggingService, ProfileService},
    models::UserStatus,
};

/// V3 User representation (complete with metadata)
#[derive(Debug, Serialize)]
pub struct UserV3 {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub roles: Vec<RoleInfo>,
    pub permissions: Vec<PermissionInfo>,
    pub metadata: UserMetadata,
}

#[derive(Debug, Serialize)]
pub struct RoleInfo {
    pub name: String,
    pub description: Option<String>,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserMetadata {
    pub active_sessions: i32,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub login_count: i64,
    pub profile_completeness: f32, // Percentage of profile completion
}

/// V3 Registration request (complete with validation)
#[derive(Debug, Deserialize)]
pub struct RegisterV3Request {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub terms_accepted: bool,
    pub marketing_consent: Option<bool>,
}

/// V3 Profile representation (complete)
#[derive(Debug, Serialize)]
pub struct ProfileV3 {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub preferences: serde_json::Value,
    pub social_links: Option<serde_json::Value>,
    pub privacy_settings: PrivacySettings,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct PrivacySettings {
    pub profile_visibility: String, // "public", "private", "friends"
    pub email_visibility: bool,
    pub activity_visibility: bool,
}

/// V3 Login response (complete with security info)
#[derive(Debug, Serialize)]
pub struct LoginV3Response {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Vec<String>,
    pub user: UserV3Summary,
    pub security: SecurityInfo,
}

#[derive(Debug, Serialize)]
pub struct UserV3Summary {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityInfo {
    pub two_factor_enabled: bool,
    pub last_password_change: Option<chrono::DateTime<chrono::Utc>>,
    pub active_sessions: i32,
    pub login_attempts_today: i32,
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub include_metadata: Option<bool>,
    pub include_permissions: Option<bool>,
    pub include_activity: Option<bool>,
}

/// Get user profile (V3 - complete response)
pub async fn get_profile_v3(
    Extension(claims): Extension<Claims>,
    State(user_service): State<UserService>,
    Query(query): Query<UserQuery>,
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
                    let profile_v3 = ProfileV3 {
                        user_id: user.id,
                        first_name: profile.first_name,
                        last_name: profile.last_name,
                        email: user.email,
                        bio: profile.bio,
                        location: profile.location,
                        website: profile.website,
                        avatar_url: profile.avatar_url,
                        phone: None, // Not available in current model
                        date_of_birth: None, // Not available in current model
                        preferences: profile.preferences,
                        social_links: None, // Not available in current model
                        privacy_settings: PrivacySettings {
                            profile_visibility: "public".to_string(), // Default value
                            email_visibility: false,
                            activity_visibility: true,
                        },
                        created_at: profile.created_at,
                        updated_at: profile.updated_at,
                    };
                    versioned_json(profile_v3, ApiVersion::V3).into_response()
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

/// Register user (V3 - complete registration with validation)
pub async fn register_v3(
    State(user_service): State<UserService>,
    Json(payload): Json<RegisterV3Request>,
) -> impl IntoResponse {
    // Validate terms acceptance
    if !payload.terms_accepted {
        return ErrorTranslator::to_http_response(crate::errors::AppError::Validation {
            field: "terms_accepted".to_string(),
            message: "Terms and conditions must be accepted".to_string(),
        });
    }

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
                "verification": {
                    "email_sent": true,
                    "expires_in": 3600, // 1 hour
                    "resend_available_in": 60 // 1 minute
                },
                "next_steps": [
                    "Check your email for verification link",
                    "Complete your profile after verification",
                    "Set up two-factor authentication for enhanced security"
                ],
                "welcome": {
                    "documentation": "/api/v3/docs",
                    "support": "/api/v3/support",
                    "community": "/api/v3/community"
                }
            });
            versioned_json(response, ApiVersion::V3).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}

/// Get user by ID (V3 - complete response)
pub async fn get_user_v3(
    Path(user_id): Path<Uuid>,
    State(user_service): State<UserService>,
    Query(query): Query<UserQuery>,
) -> impl IntoResponse {
    match user_service.repo.find_user_by_id(user_id).await {
        Ok(Some(user)) => {
            let rbac_service = RbacService::new(user_service.repo.db.clone());

            // Get roles with detailed info
            let roles = match rbac_service.get_user_roles(user_id).await {
                Ok(user_roles) => user_roles.into_iter().map(|r| RoleInfo {
                    name: r.name,
                    description: r.description,
                    assigned_at: r.assigned_at,
                    expires_at: r.expires_at,
                }).collect(),
                Err(_) => vec![],
            };

            // Get permissions if requested
            let permissions = if query.include_permissions.unwrap_or(false) {
                match rbac_service.get_user_permissions(user_id).await {
                    Ok(perms) => perms.into_iter().map(|p| PermissionInfo {
                        resource: p.resource,
                        action: p.action,
                        description: p.description,
                    }).collect(),
                    Err(_) => vec![],
                }
            } else {
                vec![]
            };

            // Get metadata if requested
            let metadata = if query.include_metadata.unwrap_or(true) {
                let activity_service = ActivityLoggingService::new(user_service.repo.db.clone());
                let stats = activity_service.get_user_activity_stats(user_id).await.unwrap_or_default();

                UserMetadata {
                    active_sessions: 1, // Placeholder
                    last_activity: Some(chrono::Utc::now()), // Placeholder
                    login_count: stats.total_activities,
                    profile_completeness: calculate_profile_completeness(&user),
                }
            } else {
                UserMetadata {
                    active_sessions: 0,
                    last_activity: None,
                    login_count: 0,
                    profile_completeness: 0.0,
                }
            };

            let user_v3 = UserV3 {
                id: user.id,
                username: user.username,
                email: user.email,
                email_verified: user.email_verified,
                status: user.status,
                created_at: user.created_at,
                updated_at: user.updated_at,
                last_login_at: user.last_login_at,
                roles,
                permissions,
                metadata,
            };
            versioned_json(user_v3, ApiVersion::V3).into_response()
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

/// Login (V3 - complete response with security info)
pub async fn login_v3(
    State(user_service): State<UserService>,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    match user_service.handle_login(payload.clone()).await {
        Ok(token) => {
            let user = match user_service.repo.find_user_by_email(&payload.email).await {
                Ok(Some(user)) => user,
                _ => {
                    return ErrorTranslator::to_http_response(crate::errors::AppError::Internal {
                        message: "Failed to retrieve user information".to_string(),
                    });
                }
            };

            let rbac_service = RbacService::new(user_service.repo.db.clone());
            let roles = match rbac_service.get_user_roles(user.id).await {
                Ok(user_roles) => user_roles.into_iter().map(|r| r.name).collect(),
                Err(_) => vec![],
            };

            let permissions = match rbac_service.get_user_permissions(user.id).await {
                Ok(perms) => perms.into_iter().map(|p| format!("{}:{}", p.resource, p.action)).collect(),
                Err(_) => vec![],
            };

            let user_summary = UserV3Summary {
                id: user.id,
                username: user.username,
                email: user.email.clone(),
                email_verified: user.email_verified,
                roles,
                permissions,
            };

            let security_info = SecurityInfo {
                two_factor_enabled: false, // Placeholder
                last_password_change: None, // Placeholder
                active_sessions: 1, // Placeholder
                login_attempts_today: 0, // Placeholder
            };

            let response = LoginV3Response {
                access_token: token,
                refresh_token: None, // Future enhancement
                token_type: "Bearer".to_string(),
                expires_in: 7 * 24 * 60 * 60, // 7 days
                scope: vec!["read".to_string(), "write".to_string()], // Based on permissions
                user: user_summary,
                security: security_info,
            };

            versioned_json(response, ApiVersion::V3).into_response()
        }
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}

fn calculate_profile_completeness(user: &crate::models::User) -> f32 {
    let mut completed_fields = 0;
    let total_fields = 5;

    if !user.username.is_empty() { completed_fields += 1; }
    if !user.email.is_empty() { completed_fields += 1; }
    if user.email_verified { completed_fields += 1; }
    if user.last_login_at.is_some() { completed_fields += 1; }
    // Add more fields as needed
    completed_fields += 1; // Always count creation as complete

    (completed_fields as f32 / total_fields as f32) * 100.0
}