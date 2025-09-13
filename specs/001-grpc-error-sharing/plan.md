# Feature 001: gRPC Error Sharing - Technical Implementation Plan

**Feature**: Improve gRPC error handling by sharing logic with Axum to avoid code duplication
**Branch**: 001-grpc-error-sharing
**Status**: Planning Phase
**Date**: 2025-09-13

## 🎯 Problem Statement

Current error handling shows inconsistency:
- **gRPC**: Uses `tonic::Status` with detailed messages like "❌ Invalid user ID format. Expected UUID format."
- **HTTP**: Uses simple strings like "User not found", "Failed to get user"
- **Service Layer**: Uses `sqlx::Error` which gets converted differently in each protocol

This leads to code duplication and inconsistent user experience.

## 🏗️ Solution Architecture

### Core Components

#### 1. Shared Error Types (`src/domain/errors.rs`)
```rust
#[derive(Debug, Clone)]
pub enum AppError {
    Validation { field: String, message: String },
    NotFound { resource: String, id: Option<String> },
    Authentication { message: String },
    Authorization { message: String },
    Database { message: String },
    Internal { message: String },
}

impl AppError {
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

    pub fn user_message(&self) -> String {
        match self {
            AppError::Validation { field, message } =>
                format!("❌ {}: {}", field, message),
            AppError::NotFound { resource, id } => {
                let id_part = id.as_ref().map(|id| format!(" with ID {}", id)).unwrap_or_default();
                format!("❌ {} not found{}", resource, id_part)
            }
            AppError::Authentication { message } =>
                format!("❌ Authentication failed: {}", message),
            AppError::Authorization { message } =>
                format!("❌ Access denied: {}", message),
            AppError::Database { .. } =>
                "❌ Database error occurred".to_string(),
            AppError::Internal { .. } =>
                "❌ Internal server error".to_string(),
        }
    }
}
```

#### 2. Error Translator Service (`src/infrastructure/errors/mod.rs`)
```rust
pub struct ErrorTranslator;

impl ErrorTranslator {
    pub fn to_grpc_status(error: AppError) -> tonic::Status {
        match error {
            AppError::Validation { .. } => tonic::Status::invalid_argument(error.user_message()),
            AppError::NotFound { .. } => tonic::Status::not_found(error.user_message()),
            AppError::Authentication { .. } => tonic::Status::unauthenticated(error.user_message()),
            AppError::Authorization { .. } => tonic::Status::permission_denied(error.user_message()),
            AppError::Database { .. } => tonic::Status::internal(error.user_message()),
            AppError::Internal { .. } => tonic::Status::internal(error.user_message()),
        }
    }

    pub fn to_http_response(error: AppError) -> axum::response::Response {
        let status_code = match error {
            AppError::Validation { .. } => axum::http::StatusCode::BAD_REQUEST,
            AppError::NotFound { .. } => axum::http::StatusCode::NOT_FOUND,
            AppError::Authentication { .. } => axum::http::StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => axum::http::StatusCode::FORBIDDEN,
            AppError::Database { .. } | AppError::Internal { .. } => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
```

#### 3. Service Layer Integration
Update `UserService` to return `Result<T, AppError>` instead of `Result<T, sqlx::Error>`:

```rust
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

## 📋 Implementation Tasks

### Phase 1: Core Infrastructure
- [ ] Create `src/domain/errors.rs` with `AppError` enum
- [ ] Create `src/infrastructure/errors/mod.rs` with `ErrorTranslator`
- [ ] Add error conversion traits for `sqlx::Error` → `AppError`
- [ ] Update `Cargo.toml` dependencies if needed

### Phase 2: Service Layer Updates
- [ ] Update `UserService::handle_get_user_by_id` to return `AppError`
- [ ] Update `UserService::handle_login` to return `AppError`
- [ ] Update `UserService::handle_create_user` to return `AppError`
- [ ] Add error conversion helpers for JWT and other service errors

### Phase 3: gRPC Integration
- [ ] Update `src/infrastructure/grpc/users.rs` to use `ErrorTranslator::to_grpc_status`
- [ ] Replace manual `Status::` calls with shared error handling
- [ ] Update all gRPC methods: `create_user`, `get_user`, `login`, `get_profile`
- [ ] Test gRPC error responses maintain current format

### Phase 4: HTTP Integration
- [ ] Update `src/infrastructure/http/controllers/user_controller.rs` to use `ErrorTranslator::to_http_response`
- [ ] Replace string responses with structured JSON error responses
- [ ] Update all HTTP endpoints: `create_user`, `get_user_by_id`, `login`, `get_profile`
- [ ] Ensure consistent error format across all endpoints

### Phase 5: Testing & Validation
- [ ] Write unit tests for `AppError` and `ErrorTranslator`
- [ ] Write integration tests for both gRPC and HTTP error scenarios
- [ ] Test error scenarios: invalid UUID, user not found, auth failures, database errors
- [ ] Verify error messages are consistent between protocols

### Phase 6: Documentation & Cleanup
- [ ] Update API documentation with new error formats
- [ ] Add error code reference documentation
- [ ] Remove duplicate error handling code
- [ ] Update README with error handling patterns

## 🔧 Technical Considerations

### Error Context Preservation
- Maintain request IDs for tracing
- Include relevant field names in validation errors
- Preserve stack traces for internal logging
- Filter sensitive information from user-facing messages

### Backward Compatibility
- Keep existing gRPC error message format (with emojis)
- Ensure HTTP clients can handle new JSON error structure
- Provide migration path for existing error handling

### Performance Impact
- Minimal overhead from error type conversions
- Reuse error objects where possible
- Avoid allocations in hot paths

## 🎨 Design Patterns Used

1. **Error as Values**: Using enum-based error types instead of exceptions
2. **Type-Driven Design**: Making invalid states unrepresentable
3. **Adapter Pattern**: `ErrorTranslator` adapts domain errors to protocol-specific formats
4. **Composition over Inheritance**: Building complex errors from simple components

## 📊 Success Metrics

- ✅ Zero code duplication in error handling logic
- ✅ Consistent error messages across gRPC and HTTP
- ✅ Structured error responses for better client handling
- ✅ Maintainable error handling code
- ✅ Comprehensive test coverage for error scenarios

## 🚀 Next Steps

1. **Immediate**: Implement core error types and translator
2. **Week 1**: Update service layer to use new error types
3. **Week 2**: Integrate with gRPC services
4. **Week 3**: Integrate with HTTP controllers
5. **Week 4**: Testing, documentation, and cleanup

## 📝 Open Questions

- Should we include error details in production vs development?
- Do we need internationalization support for error messages?
- Should we add error metrics/monitoring?
- How to handle partial success scenarios?

---

**Plan Created**: 2025-09-13
**Estimated Effort**: 2-3 weeks
**Risk Level**: Medium (changes error handling across entire application)
**Dependencies**: None