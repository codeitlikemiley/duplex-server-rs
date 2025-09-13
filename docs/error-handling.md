# Error Handling Guide

## Overview

This document describes the comprehensive error handling system implemented in the gRPC Error Sharing feature. The system provides consistent, informative error responses across both REST and gRPC APIs while eliminating code duplication.

## Architecture

### Core Components

1. **AppError Enum** (`src/domain/errors.rs`)
   - Central error type definition
   - Type-safe error categorization
   - User-friendly error messages

2. **ErrorTranslator Service** (`src/infrastructure/errors/mod.rs`)
   - Protocol-specific error conversion
   - HTTP JSON response formatting
   - gRPC Status code mapping

3. **Service Layer Integration**
   - Updated UserService to return `Result<T, AppError>`
   - Automatic error conversion from `sqlx::Error`
   - Consistent error propagation

## Error Types

### AppError Variants

```rust
pub enum AppError {
    Validation { field: String, message: String },     // Input validation errors
    NotFound { resource: String, id: Option<String> }, // Resource not found
    Authentication { message: String },                // Auth failures
    Authorization { message: String },                 // Permission errors
    Database { message: String },                      // DB operation errors
    Internal { message: String },                      // Internal server errors
}
```

### Error Code Mapping

| AppError Variant | Error Code | HTTP Status | gRPC Status |
|------------------|------------|-------------|-------------|
| Validation | VALIDATION_ERROR | 400 | INVALID_ARGUMENT |
| NotFound | NOT_FOUND | 404 | NOT_FOUND |
| Authentication | AUTHENTICATION_ERROR | 401 | UNAUTHENTICATED |
| Authorization | AUTHORIZATION_ERROR | 403 | PERMISSION_DENIED |
| Database | DATABASE_ERROR | 500 | INTERNAL |
| Internal | INTERNAL_ERROR | 500 | INTERNAL |

## Response Formats

### HTTP REST API

All HTTP errors return structured JSON responses:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "❌ Invalid user ID format. Expected UUID format.",
    "timestamp": "2025-09-13T04:12:00Z"
  }
}
```

### gRPC API

gRPC errors use standard Status codes with consistent messages:

```
Status {
  code: INVALID_ARGUMENT,
  message: "❌ Invalid user ID format. Expected UUID format."
}
```

## Usage Patterns

### For Service Methods

```rust
use crate::domain::errors::AppError;

impl UserService {
    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        self.repo.find_user_by_id(id).await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(id.to_string())
                },
                _ => AppError::Database {
                    message: "Failed to query user".to_string()
                }
            })
    }
}
```

### For HTTP Controllers

```rust
use crate::infrastructure::errors::ErrorTranslator;

pub async fn get_user_by_id(
    State(state): State<UserService>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.handle_get_user_by_id(id).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => ErrorTranslator::to_http_response(AppError::NotFound {
            resource: "User".to_string(),
            id: Some(id.to_string())
        }),
        Err(app_error) => ErrorTranslator::to_http_response(app_error),
    }
}
```

### For gRPC Handlers

```rust
use crate::infrastructure::errors::ErrorTranslator;

pub async fn get_user(
    &self,
    request: Request<GetUserRequest>,
) -> Result<Response<GetUserResponse>, Status> {
    let id_str = request.into_inner().id;

    let id = Uuid::parse_str(&id_str)
        .map_err(|_| AppError::Validation {
            field: "id".to_string(),
            message: "Invalid UUID format".to_string(),
        })?;

    match self.repo.handle_get_user_by_id(id).await {
        Ok(Some(user)) => {
            let response = Response::new(GetUserResponse {
                id: user.id.to_string(),
                username: user.username,
                email: user.email,
            });
            Ok(response)
        }
        Ok(None) => Err(ErrorTranslator::to_grpc_status(AppError::NotFound {
            resource: "User".to_string(),
            id: Some(id_str)
        })),
        Err(app_error) => Err(ErrorTranslator::to_grpc_status(app_error)),
    }
}
```

## Error Conversion Traits

### From Implementations

```rust
// Automatic conversion from sqlx::Error
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => AppError::NotFound {
                resource: "Resource".to_string(),
                id: None,
            },
            _ => AppError::Database {
                message: "Database operation failed".to_string(),
            },
        }
    }
}

// Automatic conversion from jsonwebtoken::Error
impl From<jsonwebtoken::Error> for AppError {
    fn from(error: jsonwebtoken::Error) -> Self {
        AppError::Authentication {
            message: "Invalid or expired token".to_string(),
        }
    }
}
```

## Security Considerations

### Data Filtering
- Passwords and sensitive data are never included in error messages
- Database connection strings are filtered out
- Internal stack traces are not exposed to clients
- Error messages are safe for production environments

### Information Leakage Prevention
```rust
// Safe error message
AppError::Database {
    message: "Database operation failed".to_string(),
}

// Instead of exposing internal details
// AppError::Database {
//     message: "Connection to postgresql://user:pass@host:5432/db failed".to_string(),
// }
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let error = AppError::Validation {
            field: "email".to_string(),
            message: "Invalid format".to_string(),
        };

        assert_eq!(error.code(), "VALIDATION_ERROR");
        assert!(error.user_message().contains("❌"));
    }

    #[test]
    fn test_error_translator_http() {
        let error = AppError::NotFound {
            resource: "User".to_string(),
            id: Some("123".to_string()),
        };

        let response = ErrorTranslator::to_http_response(error);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_error_consistency() {
    // Test that same error produces consistent responses
    // across HTTP and gRPC protocols
    let error = AppError::Validation {
        field: "id".to_string(),
        message: "Invalid UUID".to_string(),
    };

    let http_response = ErrorTranslator::to_http_response(error.clone());
    let grpc_status = ErrorTranslator::to_grpc_status(error);

    // Both should contain the same error message
    assert!(http_response contains "❌ Invalid UUID");
    assert!(grpc_status.message() contains "❌ Invalid UUID");
}
```

## Best Practices

### Error Message Guidelines
1. **Be Descriptive**: Include field names and expected formats
2. **Be Consistent**: Use the same message patterns across the application
3. **Be Secure**: Never include sensitive information
4. **Be User-Friendly**: Use clear, actionable language

### Error Handling Patterns
1. **Early Validation**: Validate inputs before processing
2. **Specific Errors**: Use the most specific error type available
3. **Context Preservation**: Include relevant context (field names, IDs)
4. **Logging**: Log internal errors with full context for debugging

### Performance Considerations
1. **Error Object Reuse**: Reuse error objects when possible
2. **Lazy Formatting**: Format error messages only when needed
3. **Minimal Allocations**: Avoid unnecessary string allocations in hot paths

## Migration Guide

### For Existing Services

1. **Update Return Types**
   ```rust
   // Before
   pub async fn my_method(&self) -> Result<Data, sqlx::Error>

   // After
   pub async fn my_method(&self) -> Result<Data, AppError>
   ```

2. **Update Error Handling**
   ```rust
   // Before
   match result {
       Ok(data) => Ok(data),
       Err(sqlx::Error::RowNotFound) => Err(CustomError::NotFound),
       Err(e) => Err(CustomError::Internal(e.to_string())),
   }

   // After
   result.map_err(|e| e.into()) // Automatic conversion
   ```

3. **Update Controllers**
   ```rust
   // Before
   match service_call().await {
       Ok(data) => Json(data).into_response(),
       Err(e) => "Error occurred".into_response(),
   }

   // After
   match service_call().await {
       Ok(data) => Json(data).into_response(),
       Err(app_error) => ErrorTranslator::to_http_response(app_error),
   }
   ```

## Troubleshooting

### Common Issues

1. **Compilation Errors**
   - Ensure all `From` implementations are in scope
   - Check that `AppError` is imported in service modules

2. **Runtime Errors**
   - Verify error messages don't contain sensitive data
   - Check that error codes match expected values

3. **Testing Failures**
   - Ensure test error messages match expected formats
   - Verify that error conversions work as expected

### Debug Tips

1. **Enable Debug Logging**
   ```rust
   tracing::debug!("Error occurred: {:?}", app_error);
   ```

2. **Check Error Chains**
   ```rust
   if let AppError::Database { message } = &app_error {
       tracing::error!("Database error: {}", message);
   }
   ```

3. **Validate Error Responses**
   ```rust
   // In tests
   let response = ErrorTranslator::to_http_response(error);
   assert_eq!(response.status(), StatusCode::BAD_REQUEST);
   ```

## Future Enhancements

### Planned Features
- Error metrics and monitoring
- Internationalization support for error messages
- Error correlation IDs for request tracing
- Configurable error message verbosity
- Error retry mechanisms

### Extension Points
- Custom error types for domain-specific errors
- Error middleware for additional processing
- Error serialization formats for different clients
- Error aggregation for bulk operations

---

**Last Updated**: 2025-09-13
**Version**: 1.0.0
**Feature**: gRPC Error Sharing (#001)