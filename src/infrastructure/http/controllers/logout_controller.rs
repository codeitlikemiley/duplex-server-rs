use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
    http::HeaderMap,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{SessionService, ActivityLoggingService},
    PostgreSQL,
};

/// Logout current session (single device)
pub async fn logout(
    Extension(claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Extract the token from Authorization header
    let token = match headers.get("authorization") {
        Some(auth_header) => {
            let auth_str = match auth_header.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return ErrorTranslator::to_http_response(AppError::Authentication {
                        message: "Invalid authorization header".to_string(),
                    });
                }
            };

            // Remove "Bearer " prefix
            if auth_str.starts_with("Bearer ") {
                &auth_str[7..]
            } else {
                return ErrorTranslator::to_http_response(AppError::Authentication {
                    message: "Invalid token format".to_string(),
                });
            }
        }
        None => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Missing authorization header".to_string(),
            });
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let session_service = SessionService::new(db.clone());
    let activity_service = ActivityLoggingService::new(db.pool());

    // Revoke the current session
    match session_service.revoke_session(token).await {
        Ok(()) => {
            info!("User {} logged out successfully", user_id);

            // Log the logout activity
            let _ = activity_service.log_logout(
                user_id,
                None, // IP address could be extracted from request
                None, // User agent could be extracted from headers
            ).await;

            Json(serde_json::json!({
                "success": true,
                "message": "Logged out successfully"
            }))
            .into_response()
        }
        Err(app_error) => {
            warn!("Failed to logout user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Logout from all devices
pub async fn logout_all_devices(
    Extension(claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let session_service = SessionService::new(db.clone());
    let activity_service = ActivityLoggingService::new(db.pool());

    // Revoke all user sessions
    match session_service.revoke_all_user_sessions(user_id).await {
        Ok(revoked_count) => {
            info!("User {} logged out from {} devices", user_id, revoked_count);

            // Log the logout activity
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("revoked_sessions".to_string(), serde_json::json!(revoked_count));
            let _ = activity_service.log_activity(
                crate::services::UserActivity {
                    id: uuid::Uuid::now_v7(),
                    user_id,
                    activity_type: crate::services::ActivityType::Logout,
                    description: format!("Logged out from all {} devices", revoked_count),
                    ip_address: None,
                    user_agent: None,
                    metadata,
                    created_at: chrono::Utc::now(),
                }
            ).await;

            Json(serde_json::json!({
                "success": true,
                "message": format!("Logged out from {} device(s)", revoked_count),
                "revoked_sessions": revoked_count
            }))
            .into_response()
        }
        Err(app_error) => {
            warn!("Failed to logout user {} from all devices: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get active sessions for the current user
pub async fn get_active_sessions(
    Extension(claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let session_service = SessionService::new(db.clone());

    match session_service.get_user_sessions(user_id).await {
        Ok(sessions) => {
            info!("Retrieved {} active sessions for user {}", sessions.len(), user_id);

            // Map sessions to a safe format (don't expose token hashes)
            let safe_sessions: Vec<_> = sessions.iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "ip_address": s.ip_address,
                    "user_agent": s.user_agent,
                    "expires_at": s.expires_at,
                    "last_activity_at": s.last_activity_at,
                })
            }).collect();

            Json(serde_json::json!({
                "success": true,
                "sessions": safe_sessions,
                "count": sessions.len()
            }))
            .into_response()
        }
        Err(app_error) => {
            warn!("Failed to get sessions for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Revoke a specific session by ID
pub async fn revoke_session(
    Extension(claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
    axum::extract::Path(session_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // First verify the session belongs to the user
    let result = sqlx::query!(
        r#"
        UPDATE user_sessions
        SET revoked = true
        WHERE id = $1 AND user_id = $2 AND revoked = false
        "#,
        session_id,
        user_id
    )
    .execute(&db.db)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("User {} revoked session {}", user_id, session_id);

                // Log the activity
                let activity_service = ActivityLoggingService::new(db.pool());
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("revoked_session_id".to_string(), serde_json::json!(session_id));
                let _ = activity_service.log_activity(
                    crate::services::UserActivity {
                        id: uuid::Uuid::now_v7(),
                        user_id,
                        activity_type: crate::services::ActivityType::Logout,
                        description: format!("Revoked session {}", session_id),
                        ip_address: None,
                        user_agent: None,
                        metadata,
                        created_at: chrono::Utc::now(),
                    }
                ).await;

                Json(serde_json::json!({
                    "success": true,
                    "message": "Session revoked successfully"
                }))
                .into_response()
            } else {
                ErrorTranslator::to_http_response(AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(session_id.to_string()),
                })
            }
        }
        Err(e) => {
            warn!("Failed to revoke session {} for user {}: {:?}", session_id, user_id, e);
            ErrorTranslator::to_http_response(AppError::Database {
                message: format!("Failed to revoke session: {}", e),
            })
        }
    }
}

/// Admin: Clean up expired sessions
pub async fn cleanup_sessions(
    Extension(claims): Extension<Claims>,
    State(db): State<PostgreSQL>,
) -> impl IntoResponse {
    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let session_service = SessionService::new(db.clone());

    match session_service.cleanup_expired_sessions().await {
        Ok(cleaned_count) => {
            info!("Admin {} cleaned up {} expired sessions", claims.sub, cleaned_count);

            Json(serde_json::json!({
                "success": true,
                "message": format!("Cleaned up {} expired sessions", cleaned_count),
                "cleaned_count": cleaned_count
            }))
            .into_response()
        }
        Err(app_error) => {
            warn!("Failed to cleanup sessions: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}