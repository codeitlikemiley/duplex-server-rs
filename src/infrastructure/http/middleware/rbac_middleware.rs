use axum::{
    body::Body,
    extract::{Extension, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    infrastructure::auth::Claims,
    services::RbacService,
    PostgreSQL,
};

/// Middleware to check if user has required permission
pub async fn require_permission(
    resource: &'static str,
    action: &'static str,
) -> impl Fn(Request<Body>, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request<Body>, next: Next| {
        Box::pin(async move {
            // Extract claims from extensions (should be set by auth middleware)
            let claims = match req.extensions().get::<Claims>() {
                Some(claims) => claims.clone(),
                None => {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "message": "Authentication required"
                    }))
                    .into_response();
                }
            };

            // Parse user ID
            let user_id = match Uuid::parse_str(&claims.sub) {
                Ok(id) => id,
                Err(_) => {
                    return Json(serde_json::json!({
                        "error": "Invalid token",
                        "message": "Invalid user ID in token"
                    }))
                    .into_response();
                }
            };

            // Get database connection from extensions
            let db = match req.extensions().get::<PostgreSQL>() {
                Some(db) => db.clone(),
                None => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Database connection not available"
                        })),
                    )
                        .into_response();
                }
            };

            // Check permission
            let rbac_service = RbacService::new(db.pool());
            match rbac_service.has_permission(user_id, resource, action).await {
                Ok(true) => {
                    // User has permission, continue
                    next.run(req).await
                }
                Ok(false) => {
                    // User doesn't have permission
                    Json(serde_json::json!({
                        "error": "Forbidden",
                        "message": format!("You don't have permission to {} {}", action, resource)
                    }))
                    .into_response()
                }
                Err(_) => {
                    // Error checking permission
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Failed to check permissions"
                        })),
                    )
                        .into_response()
                }
            }
        })
    }
}

/// Middleware to check if user has any of the required permissions
pub async fn require_any_permission(
    permissions: Vec<(&'static str, &'static str)>,
) -> impl Fn(Request<Body>, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request<Body>, next: Next| {
        let permissions = permissions.clone();
        Box::pin(async move {
            // Extract claims from extensions
            let claims = match req.extensions().get::<Claims>() {
                Some(claims) => claims.clone(),
                None => {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "message": "Authentication required"
                    }))
                    .into_response();
                }
            };

            // Parse user ID
            let user_id = match Uuid::parse_str(&claims.sub) {
                Ok(id) => id,
                Err(_) => {
                    return Json(serde_json::json!({
                        "error": "Invalid token",
                        "message": "Invalid user ID in token"
                    }))
                    .into_response();
                }
            };

            // Get database connection
            let db = match req.extensions().get::<PostgreSQL>() {
                Some(db) => db.clone(),
                None => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Database connection not available"
                        })),
                    )
                        .into_response();
                }
            };

            // Check permissions
            let rbac_service = RbacService::new(db.pool());
            let perms: Vec<(String, String)> = permissions
                .iter()
                .map(|(r, a)| (r.to_string(), a.to_string()))
                .collect();

            match rbac_service.has_any_permission(user_id, perms).await {
                Ok(true) => {
                    // User has at least one permission
                    next.run(req).await
                }
                Ok(false) => {
                    // User doesn't have any of the required permissions
                    Json(serde_json::json!({
                        "error": "Forbidden",
                        "message": "Insufficient permissions for this operation"
                    }))
                    .into_response()
                }
                Err(_) => {
                    // Error checking permissions
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Failed to check permissions"
                        })),
                    )
                        .into_response()
                }
            }
        })
    }
}

/// Middleware to check if user has a specific role
pub async fn require_role(
    role_name: &'static str,
) -> impl Fn(Request<Body>, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request<Body>, next: Next| {
        Box::pin(async move {
            // Extract claims from extensions
            let claims = match req.extensions().get::<Claims>() {
                Some(claims) => claims.clone(),
                None => {
                    return Json(serde_json::json!({
                        "error": "Unauthorized",
                        "message": "Authentication required"
                    }))
                    .into_response();
                }
            };

            // Parse user ID
            let user_id = match Uuid::parse_str(&claims.sub) {
                Ok(id) => id,
                Err(_) => {
                    return Json(serde_json::json!({
                        "error": "Invalid token",
                        "message": "Invalid user ID in token"
                    }))
                    .into_response();
                }
            };

            // Get database connection
            let db = match req.extensions().get::<PostgreSQL>() {
                Some(db) => db.clone(),
                None => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Database connection not available"
                        })),
                    )
                        .into_response();
                }
            };

            // Check if user has the role
            let rbac_service = RbacService::new(db.pool());
            match rbac_service.get_user_roles(user_id).await {
                Ok(roles) => {
                    if roles.iter().any(|r| r.name == role_name) {
                        // User has the required role
                        next.run(req).await
                    } else {
                        // User doesn't have the required role
                        Json(serde_json::json!({
                            "error": "Forbidden",
                            "message": format!("Requires {} role", role_name)
                        }))
                        .into_response()
                    }
                }
                Err(_) => {
                    // Error checking roles
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Internal error",
                            "message": "Failed to check user roles"
                        })),
                    )
                        .into_response()
                }
            }
        })
    }
}