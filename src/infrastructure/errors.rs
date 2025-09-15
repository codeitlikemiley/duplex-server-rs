//! Error translation service for converting domain errors to protocol-specific formats
//!
//! This module provides translators that convert AppError instances to the appropriate
//! response formats for different protocols (gRPC, HTTP).

use axum::response::{IntoResponse, Response};
use tonic::Status;

use crate::errors::AppError;

/// Service for translating domain errors to protocol-specific response formats
pub struct ErrorTranslator;

impl ErrorTranslator {
    /// Convert AppError to tonic::Status for gRPC responses
    pub fn to_grpc_status(error: AppError) -> Status {
        match error {
            AppError::Validation { .. } => Status::invalid_argument(error.user_message()),
            AppError::NotFound { .. } => Status::not_found(error.user_message()),
            AppError::Authentication { .. } => Status::unauthenticated(error.user_message()),
            AppError::Authorization { .. } => Status::permission_denied(error.user_message()),
            AppError::Database { .. } => Status::internal(error.user_message()),
            AppError::Internal { .. } => Status::internal(error.user_message()),
            AppError::Unauthorized { .. } => Status::unauthenticated(error.user_message()),
            AppError::TooManyRequests { .. } => Status::resource_exhausted(error.user_message()),
        }
    }

    /// Convert AppError to axum::Response for HTTP responses
    pub fn to_http_response(error: AppError) -> Response {
        let status_code = match error {
            AppError::Validation { .. } => axum::http::StatusCode::BAD_REQUEST,
            AppError::NotFound { .. } => axum::http::StatusCode::NOT_FOUND,
            AppError::Authentication { .. } => axum::http::StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => axum::http::StatusCode::FORBIDDEN,
            AppError::Database { .. } | AppError::Internal { .. } => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized { .. } => axum::http::StatusCode::UNAUTHORIZED,
            AppError::TooManyRequests { .. } => axum::http::StatusCode::TOO_MANY_REQUESTS,
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
    fn test_to_grpc_status_validation_error() {
        let error = AppError::Validation {
            field: "email".to_string(),
            message: "Invalid format".to_string(),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "❌ email: Invalid format");
    }

    #[test]
    fn test_to_grpc_status_not_found_error() {
        let error = AppError::NotFound {
            resource: "User".to_string(),
            id: Some("123".to_string()),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert_eq!(status.message(), "❌ User not found with ID 123");
    }

    #[test]
    fn test_to_grpc_status_authentication_error() {
        let error = AppError::Authentication {
            message: "Invalid credentials".to_string(),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert_eq!(status.message(), "❌ Authentication failed: Invalid credentials");
    }

    #[test]
    fn test_to_grpc_status_authorization_error() {
        let error = AppError::Authorization {
            message: "Insufficient permissions".to_string(),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::PermissionDenied);
        assert_eq!(status.message(), "❌ Access denied: Insufficient permissions");
    }

    #[test]
    fn test_to_grpc_status_database_error() {
        let error = AppError::Database {
            message: "Connection failed".to_string(),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(status.message(), "❌ Database error occurred");
    }

    #[test]
    fn test_to_grpc_status_internal_error() {
        let error = AppError::Internal {
            message: "Unexpected error".to_string(),
        };

        let status = ErrorTranslator::to_grpc_status(error);
        assert_eq!(status.code(), tonic::Code::Internal);
        assert_eq!(status.message(), "❌ Internal server error");
    }

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