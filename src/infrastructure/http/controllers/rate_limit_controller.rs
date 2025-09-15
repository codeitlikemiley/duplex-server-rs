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
    services::{RateLimitingService, RateLimitQuery},
};

/// Get rate limiting statistics (admin feature)
pub async fn get_rate_limit_stats(
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

    // Simple admin check - in real app, you'd have proper role checking
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let rate_limit_service = RateLimitingService::new(db.pool());

    match rate_limit_service.get_rate_limit_stats().await {
        Ok(stats) => {
            info!("Rate limit statistics retrieved by admin: {}", user_id);
            Json(serde_json::json!({
                "success": true,
                "statistics": stats
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get rate limit statistics: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get top failing IP addresses (admin feature)
pub async fn get_top_failing_ips(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<RateLimitQuery>,
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
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let rate_limit_service = RateLimitingService::new(db.pool());
    let limit = params.limit.unwrap_or(10).min(50); // Max 50 results

    match rate_limit_service.get_top_failing_ips(limit).await {
        Ok(failing_ips) => {
            info!("Top failing IPs retrieved by admin: {} (limit: {})", user_id, limit);
            Json(serde_json::json!({
                "success": true,
                "failing_ips": failing_ips,
                "limit": limit
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to get top failing IPs: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Cleanup old rate limiting records (admin feature)
pub async fn cleanup_rate_limit_records(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<RateLimitCleanupQuery>,
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
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let rate_limit_service = RateLimitingService::new(db.pool());
    let hours = params.hours.unwrap_or(168); // Default to 7 days (168 hours)

    match rate_limit_service.cleanup_old_records(hours).await {
        Ok(deleted_count) => {
            info!("Cleaned up {} old rate limit records (older than {} hours) by admin: {}", deleted_count, hours, user_id);
            Json(serde_json::json!({
                "success": true,
                "message": format!("Deleted {} old rate limit records", deleted_count),
                "deleted_count": deleted_count,
                "criteria": format!("Older than {} hours", hours)
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to cleanup old rate limit records: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Check rate limit status for a specific IP (admin feature)
pub async fn check_ip_rate_limit(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<IpRateLimitQuery>,
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
    if !claims.sub.starts_with("admin") {
        return ErrorTranslator::to_http_response(AppError::Authorization {
            message: "Admin access required".to_string(),
        });
    }

    let rate_limit_service = RateLimitingService::new(db.pool());

    match rate_limit_service.check_ip_rate_limit(&params.ip_address).await {
        Ok(rate_limit_result) => {
            info!("IP rate limit checked for {} by admin: {}", params.ip_address, user_id);
            Json(serde_json::json!({
                "success": true,
                "ip_address": params.ip_address,
                "rate_limit": rate_limit_result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to check IP rate limit for {}: {:?}", params.ip_address, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

// Query parameter DTOs
#[derive(Debug, serde::Deserialize)]
pub struct RateLimitCleanupQuery {
    pub hours: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct IpRateLimitQuery {
    pub ip_address: String,
}