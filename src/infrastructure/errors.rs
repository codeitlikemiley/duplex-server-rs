//! Error translation service for converting domain errors to protocol-specific formats
//!
//! This module provides translators that convert AppError instances to the appropriate
//! response formats for different protocols (HTTP).

use axum::response::{IntoResponse, Response};

use crate::errors::AppError;

/// Service for translating domain errors to protocol-specific response formats
pub struct ErrorTranslator;

impl ErrorTranslator {
    /// Convert AppError to axum::Response for HTTP responses
    pub fn to_http_response(error: AppError) -> Response {
        let status_code = match error {
            AppError::Validation { .. } => axum::http::StatusCode::BAD_REQUEST,
            AppError::NotFound { .. } => axum::http::StatusCode::NOT_FOUND,
            AppError::Authentication { .. } => axum::http::StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => axum::http::StatusCode::FORBIDDEN,
            AppError::Database { .. } | AppError::Internal { .. } => {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let body = serde_json::json!({
            "error": {
                "code": error.code(),
                "message": error.user_message(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        (status_code, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_to_http_response_validation_error() {
        let error = AppError::Validation {
            field: "username".to_string(),
            message: "Too short".to_string(),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Note: In a real test, you'd extract and verify the JSON body
        // For now, we just verify the status code mapping
    }

    #[test]
    fn test_to_http_response_not_found_error() {
        let error = AppError::NotFound {
            resource: "User".to_string(),
            id: None,
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_to_http_response_authentication_error() {
        let error = AppError::Authentication {
            message: "Invalid token".to_string(),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_to_http_response_authorization_error() {
        let error = AppError::Authorization {
            message: "Access denied".to_string(),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_to_http_response_database_error() {
        let error = AppError::Database {
            message: "Connection timeout".to_string(),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_to_http_response_internal_error() {
        let error = AppError::Internal {
            message: "System failure".to_string(),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
