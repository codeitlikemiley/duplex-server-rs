//! API Documentation Controller
//!
//! Provides comprehensive API documentation for the user management system,
//! including endpoint documentation, schemas, and interactive examples.

use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::PostgreSQL;

/// Get comprehensive API documentation
pub async fn get_api_documentation() -> impl IntoResponse {
    info!("API documentation requested");

    let documentation = ApiDocumentation {
        openapi: "3.0.0".to_string(),
        info: ApiInfo {
            title: "User Management API".to_string(),
            description: "Comprehensive user management system with authentication, authorization, and bulk operations".to_string(),
            version: "3.0.0".to_string(),
            contact: ApiContact {
                name: "API Support".to_string(),
                email: "api-support@quake.app".to_string(),
                url: "https://quake.app/support".to_string(),
            },
            license: ApiLicense {
                name: "MIT".to_string(),
                url: "https://opensource.org/licenses/MIT".to_string(),
            },
        },
        servers: vec![
            ApiServer {
                url: "https://api.quake.app".to_string(),
                description: "Production server".to_string(),
            },
            ApiServer {
                url: "https://staging-api.quake.app".to_string(),
                description: "Staging server".to_string(),
            },
            ApiServer {
                url: "http://localhost:80".to_string(),
                description: "Development server".to_string(),
            },
        ],
        paths: generate_api_paths(),
        components: generate_api_components(),
    };

    Json(documentation)
}

/// Get API endpoints summary
pub async fn get_api_endpoints() -> impl IntoResponse {
    info!("API endpoints summary requested");

    let endpoints = ApiEndpointsSummary {
        total_endpoints: 45,
        categories: vec![
            EndpointCategory {
                name: "Authentication".to_string(),
                description: "User authentication and session management".to_string(),
                endpoint_count: 8,
                endpoints: vec![
                    "POST /api/auth/login".to_string(),
                    "POST /api/auth/logout".to_string(),
                    "POST /api/auth/logout-all".to_string(),
                    "GET /api/auth/rate-limit-status".to_string(),
                    "GET /api/auth/sessions".to_string(),
                    "DELETE /api/auth/sessions/{session_id}".to_string(),
                    "POST /api/register".to_string(),
                    "POST /api/verify-email".to_string(),
                ],
            },
            EndpointCategory {
                name: "User Management".to_string(),
                description: "Core user CRUD operations and profile management".to_string(),
                endpoint_count: 12,
                endpoints: vec![
                    "GET /users/{id}".to_string(),
                    "POST /users".to_string(),
                    "GET /profile".to_string(),
                    "GET /api/user/profile".to_string(),
                    "PUT /api/user/profile".to_string(),
                    "DELETE /api/user/profile".to_string(),
                    "PUT /api/user/account".to_string(),
                    "POST /api/change-password".to_string(),
                    "GET /api/account/status".to_string(),
                    "POST /api/account/deactivate".to_string(),
                    "POST /api/account/reactivate".to_string(),
                    "DELETE /api/account/delete".to_string(),
                ],
            },
            EndpointCategory {
                name: "Search & Discovery".to_string(),
                description: "User search, filtering, and discovery".to_string(),
                endpoint_count: 8,
                endpoints: vec![
                    "GET /api/users/search".to_string(),
                    "POST /api/users/search/advanced".to_string(),
                    "GET /api/users/suggest".to_string(),
                    "GET /api/users/directory".to_string(),
                    "GET /api/users/statistics".to_string(),
                    "GET /api/users/paginated".to_string(),
                    "GET /api/pagination/demo".to_string(),
                    "GET /api/pagination/comparison".to_string(),
                ],
            },
            EndpointCategory {
                name: "Bulk Operations".to_string(),
                description: "Batch processing for user management".to_string(),
                endpoint_count: 5,
                endpoints: vec![
                    "POST /api/bulk/users/create".to_string(),
                    "PUT /api/bulk/users/status".to_string(),
                    "DELETE /api/bulk/users/delete".to_string(),
                    "GET /api/bulk/operations/{operation_id}/status".to_string(),
                    "GET /api/bulk/capabilities".to_string(),
                ],
            },
            EndpointCategory {
                name: "Activity & Monitoring".to_string(),
                description: "User activity tracking and system monitoring".to_string(),
                endpoint_count: 6,
                endpoints: vec![
                    "GET /api/user/activities".to_string(),
                    "GET /api/user/activities/stats".to_string(),
                    "DELETE /api/admin/activities/cleanup".to_string(),
                    "GET /api/admin/rate-limits/stats".to_string(),
                    "GET /api/admin/rate-limits/failing-ips".to_string(),
                    "DELETE /api/admin/rate-limits/cleanup".to_string(),
                ],
            },
            EndpointCategory {
                name: "Security & Administration".to_string(),
                description: "Account lockout, admin operations, and security".to_string(),
                endpoint_count: 6,
                endpoints: vec![
                    "GET /api/user/lockout-status".to_string(),
                    "GET /api/user/lockout-history".to_string(),
                    "POST /api/admin/users/{user_id}/lock".to_string(),
                    "DELETE /api/admin/users/{user_id}/unlock".to_string(),
                    "GET /api/admin/lockout/stats".to_string(),
                    "DELETE /api/admin/lockout/cleanup".to_string(),
                ],
            },
        ],
        versioning: ApiVersioning {
            current_version: "v3".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string(), "v3".to_string()],
            deprecated_versions: vec!["v1".to_string()],
            version_info: HashMap::from([
                ("v1".to_string(), "Legacy version - deprecated, will be removed in Q2 2024".to_string()),
                ("v2".to_string(), "Stable version with basic features".to_string()),
                ("v3".to_string(), "Latest version with full feature set".to_string()),
            ]),
        },
    };

    Json(endpoints)
}

/// Get API schemas and data models
pub async fn get_api_schemas() -> impl IntoResponse {
    info!("API schemas requested");

    let schemas = ApiSchemas {
        user_models: vec![
            SchemaDefinition {
                name: "User".to_string(),
                description: "Core user entity".to_string(),
                example: serde_json::json!({
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "username": "johndoe",
                    "email": "john@example.com",
                    "status": "Active",
                    "email_verified": true,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "last_login_at": "2024-01-01T00:00:00Z"
                }),
                properties: HashMap::from([
                    ("id".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: Some("uuid".to_string()),
                        description: "Unique user identifier".to_string(),
                        required: true,
                    }),
                    ("username".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: None,
                        description: "User's display name".to_string(),
                        required: true,
                    }),
                    ("email".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: Some("email".to_string()),
                        description: "User's email address".to_string(),
                        required: true,
                    }),
                    ("status".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: None,
                        description: "User account status (Active, Inactive, Suspended, PendingVerification)".to_string(),
                        required: true,
                    }),
                ]),
            },
            SchemaDefinition {
                name: "UserProfile".to_string(),
                description: "Extended user profile information".to_string(),
                example: serde_json::json!({
                    "user_id": "550e8400-e29b-41d4-a716-446655440000",
                    "first_name": "John",
                    "last_name": "Doe",
                    "bio": "Software engineer passionate about clean code",
                    "location": "San Francisco, CA",
                    "website": "https://johndoe.dev",
                    "avatar_url": "https://example.com/avatar.jpg",
                    "preferences_json": "{\"theme\": \"dark\", \"notifications\": true}",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                }),
                properties: HashMap::new(), // Simplified for brevity
            },
        ],
        request_models: vec![
            SchemaDefinition {
                name: "LoginRequest".to_string(),
                description: "User login credentials".to_string(),
                example: serde_json::json!({
                    "email": "user@example.com",
                    "password": "securepassword123"
                }),
                properties: HashMap::from([
                    ("email".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: Some("email".to_string()),
                        description: "User's email address".to_string(),
                        required: true,
                    }),
                    ("password".to_string(), PropertyDefinition {
                        r#type: "string".to_string(),
                        format: None,
                        description: "User's password (8+ characters)".to_string(),
                        required: true,
                    }),
                ]),
            },
            SchemaDefinition {
                name: "BulkCreateUsersRequest".to_string(),
                description: "Bulk user creation request".to_string(),
                example: serde_json::json!({
                    "users": [
                        {
                            "username": "user1",
                            "email": "user1@example.com",
                            "password": "password123",
                            "send_welcome_email": true
                        }
                    ],
                    "send_welcome_emails": true
                }),
                properties: HashMap::new(), // Simplified for brevity
            },
        ],
        response_models: vec![
            SchemaDefinition {
                name: "StandardResponse".to_string(),
                description: "Standard API response wrapper".to_string(),
                example: serde_json::json!({
                    "success": true,
                    "data": {},
                    "message": "Operation completed successfully",
                    "timestamp": "2024-01-01T00:00:00Z"
                }),
                properties: HashMap::from([
                    ("success".to_string(), PropertyDefinition {
                        r#type: "boolean".to_string(),
                        format: None,
                        description: "Indicates if the operation was successful".to_string(),
                        required: true,
                    }),
                    ("data".to_string(), PropertyDefinition {
                        r#type: "object".to_string(),
                        format: None,
                        description: "Response data payload".to_string(),
                        required: false,
                    }),
                ]),
            },
        ],
        error_models: vec![
            SchemaDefinition {
                name: "ErrorResponse".to_string(),
                description: "Standard error response".to_string(),
                example: serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "VALIDATION_ERROR",
                        "message": "Invalid email format",
                        "field": "email",
                        "timestamp": "2024-01-01T00:00:00Z"
                    }
                }),
                properties: HashMap::new(), // Simplified for brevity
            },
        ],
    };

    Json(schemas)
}

/// Get authentication and authorization documentation
pub async fn get_auth_documentation() -> impl IntoResponse {
    info!("Authentication documentation requested");

    let auth_docs = AuthDocumentation {
        overview: "The API uses JWT-based authentication with role-based access control (RBAC)".to_string(),
        authentication_methods: vec![
            AuthMethod {
                name: "JWT Bearer Token".to_string(),
                description: "Include JWT token in Authorization header".to_string(),
                header_format: "Authorization: Bearer <token>".to_string(),
                example: "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
            },
        ],
        token_lifecycle: TokenLifecycle {
            access_token_ttl: "1 hour".to_string(),
            refresh_token_ttl: "30 days".to_string(),
            token_refresh_endpoint: "/api/auth/refresh".to_string(),
            logout_endpoint: "/api/auth/logout".to_string(),
        },
        rate_limiting: RateLimitingInfo {
            description: "API endpoints are protected by rate limiting to prevent abuse".to_string(),
            limits: vec![
                RateLimit {
                    endpoint_pattern: "/api/auth/*".to_string(),
                    limit: "5 requests per minute per IP".to_string(),
                    description: "Authentication endpoints are strictly limited".to_string(),
                },
                RateLimit {
                    endpoint_pattern: "/api/bulk/*".to_string(),
                    limit: "2 requests per minute per user".to_string(),
                    description: "Bulk operations are heavily limited due to resource usage".to_string(),
                },
                RateLimit {
                    endpoint_pattern: "/api/*".to_string(),
                    limit: "100 requests per minute per user".to_string(),
                    description: "General API endpoints".to_string(),
                },
            ],
            headers: vec![
                "X-RateLimit-Limit: Request limit".to_string(),
                "X-RateLimit-Remaining: Remaining requests".to_string(),
                "X-RateLimit-Reset: Reset timestamp".to_string(),
            ],
        },
        error_codes: vec![
            ErrorCode {
                code: "AUTH_001".to_string(),
                name: "MISSING_TOKEN".to_string(),
                description: "Authorization header is missing".to_string(),
                http_status: 401,
            },
            ErrorCode {
                code: "AUTH_002".to_string(),
                name: "INVALID_TOKEN".to_string(),
                description: "JWT token is invalid or expired".to_string(),
                http_status: 401,
            },
            ErrorCode {
                code: "AUTH_003".to_string(),
                name: "INSUFFICIENT_PERMISSIONS".to_string(),
                description: "User lacks required permissions for this operation".to_string(),
                http_status: 403,
            },
        ],
    };

    Json(auth_docs)
}

/// Get interactive API examples
pub async fn get_api_examples() -> impl IntoResponse {
    info!("API examples requested");

    let examples = ApiExamples {
        common_workflows: vec![
            WorkflowExample {
                name: "User Registration and Login".to_string(),
                description: "Complete flow from user registration to authenticated access".to_string(),
                steps: vec![
                    ApiStep {
                        step: 1,
                        description: "Register new user".to_string(),
                        method: "POST".to_string(),
                        endpoint: "/api/register".to_string(),
                        request_body: serde_json::json!({
                            "username": "newuser",
                            "email": "newuser@example.com",
                            "password": "SecurePass123!"
                        }),
                        expected_response: serde_json::json!({
                            "success": true,
                            "message": "Registration successful. Please check your email for verification."
                        }),
                    },
                    ApiStep {
                        step: 2,
                        description: "Verify email address".to_string(),
                        method: "POST".to_string(),
                        endpoint: "/api/verify-email".to_string(),
                        request_body: serde_json::json!({
                            "email": "newuser@example.com",
                            "verification_code": "123456"
                        }),
                        expected_response: serde_json::json!({
                            "success": true,
                            "message": "Email verified successfully. You can now login."
                        }),
                    },
                    ApiStep {
                        step: 3,
                        description: "Login with credentials".to_string(),
                        method: "POST".to_string(),
                        endpoint: "/api/auth/login".to_string(),
                        request_body: serde_json::json!({
                            "email": "newuser@example.com",
                            "password": "SecurePass123!"
                        }),
                        expected_response: serde_json::json!({
                            "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
                        }),
                    },
                ],
            },
            WorkflowExample {
                name: "Bulk User Management".to_string(),
                description: "Create multiple users and manage their status in bulk".to_string(),
                steps: vec![
                    ApiStep {
                        step: 1,
                        description: "Create multiple users at once".to_string(),
                        method: "POST".to_string(),
                        endpoint: "/api/bulk/users/create".to_string(),
                        request_body: serde_json::json!({
                            "users": [
                                {
                                    "username": "bulk_user1",
                                    "email": "bulk1@example.com",
                                    "password": "TempPass123!"
                                },
                                {
                                    "username": "bulk_user2",
                                    "email": "bulk2@example.com",
                                    "password": "TempPass123!"
                                }
                            ],
                            "send_welcome_emails": true
                        }),
                        expected_response: serde_json::json!({
                            "success": true,
                            "data": {
                                "operation_id": "550e8400-e29b-41d4-a716-446655440000",
                                "total_requested": 2,
                                "successful_count": 2,
                                "failed_count": 0,
                                "created_users": [
                                    {
                                        "index": 0,
                                        "user_id": "user-id-1",
                                        "username": "bulk_user1",
                                        "email": "bulk1@example.com",
                                        "status": "created"
                                    }
                                ]
                            }
                        }),
                    },
                ],
            },
        ],
        curl_examples: vec![
            CurlExample {
                name: "Basic Login".to_string(),
                description: "Authenticate user and get JWT token".to_string(),
                command: r#"curl -X POST https://api.quake.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'"#.to_string(),
            },
            CurlExample {
                name: "Get User Profile".to_string(),
                description: "Retrieve authenticated user's profile".to_string(),
                command: r#"curl -X GET https://api.quake.app/profile \
  -H "Authorization: Bearer YOUR_JWT_TOKEN""#.to_string(),
            },
            CurlExample {
                name: "Search Users with Pagination".to_string(),
                description: "Search users with cursor-based pagination".to_string(),
                command: r#"curl -X GET "https://api.quake.app/api/users/paginated?cursor=START&direction=forward&limit=20" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN""#.to_string(),
            },
        ],
        postman_collection: PostmanCollection {
            name: "User Management API".to_string(),
            description: "Complete Postman collection for the User Management API".to_string(),
            download_url: "/api/docs/postman-collection.json".to_string(),
            variables: vec![
                PostmanVariable {
                    name: "baseUrl".to_string(),
                    description: "API base URL".to_string(),
                    default_value: "https://api.quake.app".to_string(),
                },
                PostmanVariable {
                    name: "authToken".to_string(),
                    description: "JWT authentication token".to_string(),
                    default_value: "".to_string(),
                },
            ],
        },
    };

    Json(examples)
}

/// Get Postman collection for API testing
pub async fn get_postman_collection() -> impl IntoResponse {
    info!("Postman collection requested");

    // Return a basic Postman collection structure
    let collection = serde_json::json!({
        "info": {
            "name": "User Management API",
            "description": "Complete API collection for user management system",
            "version": "3.0.0",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "variable": [
            {
                "key": "baseUrl",
                "value": "https://api.quake.app",
                "type": "string"
            },
            {
                "key": "authToken",
                "value": "",
                "type": "string"
            }
        ],
        "item": [
            {
                "name": "Authentication",
                "item": [
                    {
                        "name": "Login",
                        "request": {
                            "method": "POST",
                            "header": [
                                {
                                    "key": "Content-Type",
                                    "value": "application/json"
                                }
                            ],
                            "body": {
                                "mode": "raw",
                                "raw": serde_json::to_string_pretty(&serde_json::json!({
                                    "email": "user@example.com",
                                    "password": "password123"
                                })).unwrap()
                            },
                            "url": {
                                "raw": "{{baseUrl}}/api/auth/login",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "auth", "login"]
                            }
                        }
                    }
                ]
            }
        ]
    });

    (StatusCode::OK, Json(collection)).into_response()
}

// Data structures for API documentation
#[derive(Debug, Serialize)]
struct ApiDocumentation {
    openapi: String,
    info: ApiInfo,
    servers: Vec<ApiServer>,
    paths: HashMap<String, PathItem>,
    components: Components,
}

#[derive(Debug, Serialize)]
struct ApiInfo {
    title: String,
    description: String,
    version: String,
    contact: ApiContact,
    license: ApiLicense,
}

#[derive(Debug, Serialize)]
struct ApiContact {
    name: String,
    email: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct ApiLicense {
    name: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct ApiServer {
    url: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct PathItem {
    get: Option<Operation>,
    post: Option<Operation>,
    put: Option<Operation>,
    delete: Option<Operation>,
}

#[derive(Debug, Serialize)]
struct Operation {
    summary: String,
    description: String,
    tags: Vec<String>,
    parameters: Vec<Parameter>,
    responses: HashMap<String, Response>,
}

#[derive(Debug, Serialize)]
struct Parameter {
    name: String,
    r#in: String, // "query", "path", "header"
    description: String,
    required: bool,
    schema: Schema,
}

#[derive(Debug, Serialize)]
struct Response {
    description: String,
    content: HashMap<String, MediaType>,
}

#[derive(Debug, Serialize)]
struct MediaType {
    schema: Schema,
}

#[derive(Debug, Serialize)]
struct Schema {
    r#type: String,
    format: Option<String>,
    properties: HashMap<String, Schema>,
}

#[derive(Debug, Serialize)]
struct Components {
    schemas: HashMap<String, Schema>,
    security_schemes: HashMap<String, SecurityScheme>,
}

#[derive(Debug, Serialize)]
struct SecurityScheme {
    r#type: String,
    scheme: String,
    bearer_format: String,
}

#[derive(Debug, Serialize)]
struct ApiEndpointsSummary {
    total_endpoints: u32,
    categories: Vec<EndpointCategory>,
    versioning: ApiVersioning,
}

#[derive(Debug, Serialize)]
struct EndpointCategory {
    name: String,
    description: String,
    endpoint_count: u32,
    endpoints: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ApiVersioning {
    current_version: String,
    supported_versions: Vec<String>,
    deprecated_versions: Vec<String>,
    version_info: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct ApiSchemas {
    user_models: Vec<SchemaDefinition>,
    request_models: Vec<SchemaDefinition>,
    response_models: Vec<SchemaDefinition>,
    error_models: Vec<SchemaDefinition>,
}

#[derive(Debug, Serialize)]
struct SchemaDefinition {
    name: String,
    description: String,
    example: serde_json::Value,
    properties: HashMap<String, PropertyDefinition>,
}

#[derive(Debug, Serialize)]
struct PropertyDefinition {
    r#type: String,
    format: Option<String>,
    description: String,
    required: bool,
}

#[derive(Debug, Serialize)]
struct AuthDocumentation {
    overview: String,
    authentication_methods: Vec<AuthMethod>,
    token_lifecycle: TokenLifecycle,
    rate_limiting: RateLimitingInfo,
    error_codes: Vec<ErrorCode>,
}

#[derive(Debug, Serialize)]
struct AuthMethod {
    name: String,
    description: String,
    header_format: String,
    example: String,
}

#[derive(Debug, Serialize)]
struct TokenLifecycle {
    access_token_ttl: String,
    refresh_token_ttl: String,
    token_refresh_endpoint: String,
    logout_endpoint: String,
}

#[derive(Debug, Serialize)]
struct RateLimitingInfo {
    description: String,
    limits: Vec<RateLimit>,
    headers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RateLimit {
    endpoint_pattern: String,
    limit: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ErrorCode {
    code: String,
    name: String,
    description: String,
    http_status: u16,
}

#[derive(Debug, Serialize)]
struct ApiExamples {
    common_workflows: Vec<WorkflowExample>,
    curl_examples: Vec<CurlExample>,
    postman_collection: PostmanCollection,
}

#[derive(Debug, Serialize)]
struct WorkflowExample {
    name: String,
    description: String,
    steps: Vec<ApiStep>,
}

#[derive(Debug, Serialize)]
struct ApiStep {
    step: u32,
    description: String,
    method: String,
    endpoint: String,
    request_body: serde_json::Value,
    expected_response: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct CurlExample {
    name: String,
    description: String,
    command: String,
}

#[derive(Debug, Serialize)]
struct PostmanCollection {
    name: String,
    description: String,
    download_url: String,
    variables: Vec<PostmanVariable>,
}

#[derive(Debug, Serialize)]
struct PostmanVariable {
    name: String,
    description: String,
    default_value: String,
}

// Helper functions
fn generate_api_paths() -> HashMap<String, PathItem> {
    // Simplified implementation - in production, this would be much more comprehensive
    HashMap::from([
        ("/api/auth/login".to_string(), PathItem {
            post: Some(Operation {
                summary: "User Login".to_string(),
                description: "Authenticate user with email and password".to_string(),
                tags: vec!["Authentication".to_string()],
                parameters: vec![],
                responses: HashMap::from([
                    ("200".to_string(), Response {
                        description: "Login successful".to_string(),
                        content: HashMap::new(),
                    }),
                    ("401".to_string(), Response {
                        description: "Invalid credentials".to_string(),
                        content: HashMap::new(),
                    }),
                ]),
            }),
            get: None,
            put: None,
            delete: None,
        }),
    ])
}

fn generate_api_components() -> Components {
    Components {
        schemas: HashMap::new(),
        security_schemes: HashMap::from([
            ("bearerAuth".to_string(), SecurityScheme {
                r#type: "http".to_string(),
                scheme: "bearer".to_string(),
                bearer_format: "JWT".to_string(),
            }),
        ]),
    }
}