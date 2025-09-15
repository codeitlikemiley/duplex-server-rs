use axum::{
    Json,
    extract::{Extension, State, Query},
    response::IntoResponse,
};
use tracing::{error, info};
use uuid::Uuid;
use serde::Deserialize;

use crate::{
    errors::AppError,
    infrastructure::auth::Claims,
    infrastructure::errors::ErrorTranslator,
    services::{SearchService, UserSearchFilters, AdvancedSearchFilters},
};

/// Search users with basic filters
pub async fn search_users(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint
    State(db): State<crate::PostgreSQL>,
    Query(filters): Query<UserSearchFilters>,
) -> impl IntoResponse {
    let search_service = SearchService::new(db.pool(), db.clone());

    match search_service.search_users(filters).await {
        Ok(result) => {
            info!("User search completed: {} results found", result.users.len());
            Json(serde_json::json!({
                "success": true,
                "data": result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("User search failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Advanced search with more sophisticated filtering
pub async fn advanced_search(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint
    State(db): State<crate::PostgreSQL>,
    Json(filters): Json<AdvancedSearchFilters>,
) -> impl IntoResponse {
    let search_service = SearchService::new(db.pool(), db.clone());

    match search_service.advanced_search(filters).await {
        Ok(result) => {
            info!("Advanced search completed: {} results found", result.users.len());
            Json(serde_json::json!({
                "success": true,
                "data": result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Advanced search failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get user suggestions for autocomplete
pub async fn suggest_users(
    Extension(_claims): Extension<Claims>, // Authenticated endpoint
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<UserSuggestionQuery>,
) -> impl IntoResponse {
    if params.q.len() < 2 {
        return Json(serde_json::json!({
            "success": false,
            "message": "Query must be at least 2 characters"
        }))
        .into_response();
    }

    let search_service = SearchService::new(db.pool(), db.clone());
    let limit = params.limit.unwrap_or(10).min(20); // Max 20 suggestions

    match search_service.suggest_users(&params.q, limit).await {
        Ok(suggestions) => {
            info!("User suggestions generated: {} suggestions", suggestions.len());
            Json(serde_json::json!({
                "success": true,
                "suggestions": suggestions
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("User suggestions failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Public user directory (limited info, no authentication required)
pub async fn public_user_directory(
    State(db): State<crate::PostgreSQL>,
    Query(params): Query<PublicDirectoryQuery>,
) -> impl IntoResponse {
    // For public directory, we only show active, verified users with limited info
    let filters = UserSearchFilters {
        search: params.search,
        status: Some(crate::models::UserStatus::Active),
        email_verified: Some(true),
        created_after: None,
        created_before: None,
        location: params.location,
        sort_by: params.sort_by,
        sort_desc: params.sort_desc,
        limit: Some(params.limit.unwrap_or(20).min(50)), // Max 50 for public
        offset: params.offset,
    };

    let search_service = SearchService::new(db.pool(), db.clone());

    match search_service.search_users(filters).await {
        Ok(mut result) => {
            // Remove sensitive info for public directory
            for user in &mut result.users {
                user.email = format!("{}@***", user.email.split('@').next().unwrap_or("user"));
            }

            info!("Public directory search completed: {} results", result.users.len());
            Json(serde_json::json!({
                "success": true,
                "data": result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Public directory search failed: {:?}", app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get search statistics (admin feature)
pub async fn search_statistics(
    Extension(claims): Extension<Claims>,
    State(db): State<crate::PostgreSQL>,
) -> impl IntoResponse {
    // Simple admin check - in real app, you'd have proper role checking
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return ErrorTranslator::to_http_response(AppError::Authentication {
                message: "Invalid user ID in token".to_string(),
            });
        }
    };

    // Get basic statistics
    let stats_query = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total_users,
            COUNT(CASE WHEN status = 'Active' THEN 1 END) as active_users,
            COUNT(CASE WHEN status = 'Inactive' THEN 1 END) as inactive_users,
            COUNT(CASE WHEN status = 'Suspended' THEN 1 END) as suspended_users,
            COUNT(CASE WHEN email_verified = true THEN 1 END) as verified_users,
            COUNT(CASE WHEN created_at >= NOW() - INTERVAL '30 days' THEN 1 END) as new_users_30d,
            COUNT(CASE WHEN last_login_at >= NOW() - INTERVAL '7 days' THEN 1 END) as active_users_7d
        FROM users
        "#
    );

    match stats_query.fetch_one(&db.pool()).await {
        Ok(stats) => {
            info!("Search statistics requested by user: {}", user_id);
            Json(serde_json::json!({
                "success": true,
                "statistics": {
                    "total_users": stats.total_users,
                    "active_users": stats.active_users,
                    "inactive_users": stats.inactive_users,
                    "suspended_users": stats.suspended_users,
                    "verified_users": stats.verified_users,
                    "new_users_last_30_days": stats.new_users_30d,
                    "active_users_last_7_days": stats.active_users_7d
                }
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to get search statistics: {:?}", e);
            ErrorTranslator::to_http_response(AppError::Database {
                message: "Failed to get statistics".to_string()
            })
        }
    }
}

// Query parameter DTOs
#[derive(Debug, Deserialize)]
pub struct UserSuggestionQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PublicDirectoryQuery {
    pub search: Option<String>,
    pub location: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i64>,
}