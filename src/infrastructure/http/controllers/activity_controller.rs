use axum::{
    Json,
    extract::{Extension, State, Query},
    response::IntoResponse,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{ActivityLoggingService, ActivityFilters},
};

/// Get user's activity history
pub async fn get_user_activities(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(filters): Query<ActivityFilters>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    let activity_service = ActivityLoggingService::new(db.pool());

    match activity_service.get_user_activities(user_id, filters).await {
        Ok(result) => {
            info!("User activities retrieved for user: {}, count: {}", user_id, result.activities.len());
            Json(serde_json::json!({
                "success": true,
                "data": result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get user activities for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get user's activity statistics
pub async fn get_user_activity_stats(
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

    let activity_service = ActivityLoggingService::new(db.pool());

    match activity_service.get_user_activity_stats(user_id).await {
        Ok(stats) => {
            info!("Activity statistics retrieved for user: {}", user_id);
            Json(serde_json::json!({
                "success": true,
                "statistics": stats
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get activity statistics for user {}: {:?}", user_id, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Admin endpoint to cleanup old activities
pub async fn cleanup_old_activities(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<CleanupQuery>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Simple admin check - in real app, you'd have proper role checking
    // This is a placeholder for now
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let activity_service = ActivityLoggingService::new(db.pool());
    let days = params.days.unwrap_or(90); // Default to 90 days

    match activity_service.cleanup_old_activities(days).await {
        Ok(deleted_count) => {
            info!("Cleaned up {} old activities (older than {} days) by admin: {}", deleted_count, days, user_id);
            Json(serde_json::json!({
                "success": true,
                "message": format!("Deleted {} old activity records", deleted_count),
                "deleted_count": deleted_count,
                "criteria": format!("Older than {} days", days)
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to cleanup old activities: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

// Query parameter DTOs
#[derive(Debug, serde::Deserialize)]
pub struct CleanupQuery {
    pub days: Option<i32>,
}