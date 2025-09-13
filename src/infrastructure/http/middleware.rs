use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::warn;

use crate::infrastructure::auth::JwtService;

pub async fn auth_middleware(mut request: Request, next: Next) -> Response {
    let jwt_service = JwtService::default();

    // Extract token from Authorization header
    let auth_header = request.headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").unwrap_or("")
        }
        _ => {
            warn!("Missing or invalid Authorization header");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    // Verify token
    match jwt_service.verify_token(token) {
        Ok(claims) => {
            // Store claims in request extensions for handlers to access
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(e) => {
            warn!("Invalid JWT token: {:?}", e);
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}