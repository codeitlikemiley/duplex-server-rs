//! Shared error types for the application
//!
//! This module defines the core error types used throughout the application
//! to ensure consistent error handling across gRPC and HTTP APIs.

use std::fmt;

/// Core application error type
#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    /// Validation errors (e.g., invalid input format)
    Validation {
        field: String,
        message: String,
    },

    /// Resource not found errors
    NotFound {
        resource: String,
        id: Option<String>,
    },

    /// Authentication errors
    Authentication {
        message: String,
    },

    /// Authorization errors
    Authorization {
        message: String,
    },

    /// Database operation errors
    Database {
        message: String,
    },

    /// Internal server errors
    Internal {
        message: String,
    },
}

impl AppError {
    /// Returns the error code for this error type
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Authentication { .. } => "AUTHENTICATION_ERROR",
            AppError::Authorization { .. } => "AUTHORIZATION_ERROR",
            AppError::Database { .. } => "DATABASE_ERROR",
            AppError::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    /// Returns a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            AppError::Validation { field, message } => {
                format!("❌ {}: {}", field, message)
            }
            AppError::NotFound { resource, id } => {
                let id_part = id.as_ref()
                    .map(|id| format!(" with ID {}", id))
                    .unwrap_or_default();
                format!("❌ {} not found{}", resource, id_part)
            }
            AppError::Authentication { message } => {
                format!("❌ Authentication failed: {}", message)
            }
            AppError::Authorization { message } => {
                format!("❌ Access denied: {}", message)
            }
            AppError::Database { .. } => {
                "❌ Database error occurred".to_string()
            }
            AppError::Internal { .. } => {
                "❌ Internal server error".to_string()
            }
        }
    }

    /// Returns a detailed error message for debugging (includes sensitive info)
    pub fn debug_message(&self) -> String {
        match self {
            AppError::Validation { field, message } => {
                format!("Validation error in field '{}': {}", field, message)
            }
            AppError::NotFound { resource, id } => {
                let id_part = id.as_ref()
                    .map(|id| format!(" (ID: {})", id))
                    .unwrap_or_default();
                format!("Resource '{}' not found{}", resource, id_part)
            }
            AppError::Authentication { message } => {
                format!("Authentication failed: {}", message)
            }
            AppError::Authorization { message } => {
                format!("Authorization failed: {}", message)
            }
            AppError::Database { message } => {
                format!("Database error: {}", message)
            }
            AppError::Internal { message } => {
                format!("Internal error: {}", message)
            }
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AppError {}

/// Conversion from sqlx::Error to AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound {
                resource: "Resource".to_string(),
                id: None,
            },
            _ => AppError::Database {
                message: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let error = AppError::Validation {
            field: "email".to_string(),
            message: "Invalid format".to_string(),
        };

        assert_eq!(error.code(), "VALIDATION_ERROR");
        assert_eq!(error.user_message(), "❌ email: Invalid format");
        assert_eq!(error.debug_message(), "Validation error in field 'email': Invalid format");
    }

    #[test]
    fn test_not_found_error() {
        let error = AppError::NotFound {
            resource: "User".to_string(),
            id: Some("123".to_string()),
        };

        assert_eq!(error.code(), "NOT_FOUND");
        assert_eq!(error.user_message(), "❌ User not found with ID 123");
        assert_eq!(error.debug_message(), "Resource 'User' not found (ID: 123)");
    }

    #[test]
    fn test_not_found_error_no_id() {
        let error = AppError::NotFound {
            resource: "User".to_string(),
            id: None,
        };

        assert_eq!(error.user_message(), "❌ User not found");
        assert_eq!(error.debug_message(), "Resource 'User' not found");
    }

    #[test]
    fn test_authentication_error() {
        let error = AppError::Authentication {
            message: "Invalid credentials".to_string(),
        };

        assert_eq!(error.code(), "AUTHENTICATION_ERROR");
        assert_eq!(error.user_message(), "❌ Authentication failed: Invalid credentials");
        assert_eq!(error.debug_message(), "Authentication failed: Invalid credentials");
    }

    #[test]
    fn test_authorization_error() {
        let error = AppError::Authorization {
            message: "Insufficient permissions".to_string(),
        };

        assert_eq!(error.code(), "AUTHORIZATION_ERROR");
        assert_eq!(error.user_message(), "❌ Access denied: Insufficient permissions");
        assert_eq!(error.debug_message(), "Authorization failed: Insufficient permissions");
    }

    #[test]
    fn test_database_error() {
        let error = AppError::Database {
            message: "Connection failed".to_string(),
        };

        assert_eq!(error.code(), "DATABASE_ERROR");
        assert_eq!(error.user_message(), "❌ Database error occurred");
        assert_eq!(error.debug_message(), "Database error: Connection failed");
    }

    #[test]
    fn test_internal_error() {
        let error = AppError::Internal {
            message: "Unexpected error".to_string(),
        };

        assert_eq!(error.code(), "INTERNAL_ERROR");
        assert_eq!(error.user_message(), "❌ Internal server error");
        assert_eq!(error.debug_message(), "Internal error: Unexpected error");
    }

    #[test]
    fn test_sqlx_error_conversion_row_not_found() {
        let sqlx_error = sqlx::Error::RowNotFound;
        let app_error: AppError = sqlx_error.into();

        match app_error {
            AppError::NotFound { resource, id } => {
                assert_eq!(resource, "Resource");
                assert_eq!(id, None);
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_sqlx_error_conversion_other() {
        let sqlx_error = sqlx::Error::Configuration("Test error".into());
        let app_error: AppError = sqlx_error.into();

        match app_error {
            AppError::Database { message } => {
                assert!(message.contains("Test error"));
            }
            _ => panic!("Expected Database error"),
        }
    }

    #[test]
    fn test_display_trait() {
        let error = AppError::Validation {
            field: "username".to_string(),
            message: "Too short".to_string(),
        };

        assert_eq!(format!("{}", error), "❌ username: Too short");
    }

    #[test]
    fn test_clone_trait() {
        let error = AppError::Validation {
            field: "email".to_string(),
            message: "Invalid".to_string(),
        };

        let cloned = error.clone();
        assert_eq!(error, cloned);
    }
}