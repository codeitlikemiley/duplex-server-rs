use tonic::{Request, Status};
use tracing::warn;

use crate::infrastructure::auth::{Claims, JwtService};

pub fn auth_interceptor(mut request: Request<()>) -> Result<Request<()>, Status> {
    let jwt_service = JwtService::default();

    // Extract token from authorization metadata
    let token = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth_header| {
            if auth_header.starts_with("Bearer ") {
                Some(auth_header.strip_prefix("Bearer ").unwrap_or(""))
            } else {
                None
            }
        });

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => {
            warn!("Missing or invalid authorization header");
            return Err(Status::unauthenticated(
                "Missing or invalid authorization header",
            ));
        }
    };

    // Verify token
    match jwt_service.verify_token(token) {
        Ok(claims) => {
            // Store claims in request extensions
            request.extensions_mut().insert(claims);
            Ok(request)
        }
        Err(e) => {
            warn!("Invalid JWT token: {:?}", e);
            Err(Status::unauthenticated("Invalid token"))
        }
    }
}

pub fn get_claims<T>(request: &Request<T>) -> Result<&Claims, Status> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| Status::internal("Claims not found in request"))
}
