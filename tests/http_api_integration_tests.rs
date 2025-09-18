use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
    Router,
};
use tower::ServiceExt;
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

mod integration_test_helpers;
use integration_test_helpers::{IntegrationTestContext, TestDatabase};

use coqrs::{
    models::{User, UserStatus},
    infrastructure::http::router::router,
    commands::CommandMessage,
    PostgreSQL,
    services::UserService,
};

/// HTTP API test client with real Axum router
struct ApiTestClient {
    router: Router,
    db_context: Arc<TestDatabase>,
    _command_sender: mpsc::Sender<CommandMessage>,
}

impl ApiTestClient {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_context = Arc::new(TestDatabase::new().await?);
        let pool = db_context.pool.clone();

        // Create command channel
        let (command_sender, _command_receiver) = mpsc::channel(100);

        // Create the actual router with real services
        let router = router(pool, command_sender.clone());

        Ok(Self {
            router,
            db_context,
            _command_sender: command_sender,
        })
    }

    async fn request(&self, method: Method, uri: &str, body: Option<Value>) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        let mut req_builder = Request::builder()
            .method(method)
            .uri(uri);

        let body = if let Some(json_body) = body {
            req_builder = req_builder.header("content-type", "application/json");
            Body::from(json_body.to_string())
        } else {
            Body::empty()
        };

        let request = req_builder.body(body)?;

        let response = self.router
            .clone()
            .oneshot(request)
            .await?;

        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let json: Value = serde_json::from_slice(&body).unwrap_or(json!({}));

        Ok((status, json))
    }

    async fn get(&self, uri: &str) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        self.request(Method::GET, uri, None).await
    }

    async fn post(&self, uri: &str, body: Value) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        self.request(Method::POST, uri, Some(body)).await
    }

    async fn put(&self, uri: &str, body: Value) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        self.request(Method::PUT, uri, Some(body)).await
    }

    async fn delete(&self, uri: &str) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        self.request(Method::DELETE, uri, None).await
    }
}

/// Test user registration via HTTP API
#[tokio::test]
async fn test_user_registration_api() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test successful user registration
    let registration_payload = json!({
        "username": "httpuser",
        "email": "httpuser@example.com",
        "password": "SecurePass123!",
        "first_name": "Http",
        "last_name": "User"
    });

    let (status, response) = client
        .post("/api/register", registration_payload)
        .await
        .expect("Failed to make registration request");

    assert_eq!(status, StatusCode::OK);
    assert!(response["message"].as_str().unwrap().contains("Registration successful"));

    // Verify user was created in database
    let found_user = client.db_context.postgres
        .find_user_by_email("httpuser@example.com")
        .await
        .expect("Failed to query user");

    assert!(found_user.is_some());
    let user = found_user.unwrap();
    assert_eq!(user.username, "httpuser");
    assert_eq!(user.email, "httpuser@example.com");
    assert!(!user.email_verified); // Should start unverified
}

/// Test user registration validation via HTTP API
#[tokio::test]
async fn test_user_registration_validation() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test missing required fields
    let invalid_payload = json!({
        "username": "testuser"
        // Missing email and password
    });

    let (status, _response) = client
        .post("/api/register", invalid_payload)
        .await
        .expect("Failed to make request");

    // Should return 400 or 422 for validation error
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY);

    // Test invalid email format
    let invalid_email_payload = json!({
        "username": "testuser",
        "email": "invalid-email",
        "password": "SecurePass123!"
    });

    let (status, _response) = client
        .post("/api/register", invalid_email_payload)
        .await
        .expect("Failed to make request");

    assert!(status.is_client_error());

    // Test duplicate registration
    let duplicate_payload = json!({
        "username": "duplicateuser",
        "email": "duplicate@example.com",
        "password": "SecurePass123!"
    });

    // First registration should succeed
    let (status, _response) = client
        .post("/api/register", duplicate_payload.clone())
        .await
        .expect("Failed to make first registration");

    assert_eq!(status, StatusCode::OK);

    // Second registration with same email should fail
    let (status, _response) = client
        .post("/api/register", duplicate_payload)
        .await
        .expect("Failed to make duplicate registration");

    assert!(status.is_client_error()); // Should return 409 Conflict or similar
}

/// Test user retrieval via HTTP API
#[tokio::test]
async fn test_user_retrieval_api() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Create a test user through the database
    let ctx = IntegrationTestContext::new().await.expect("Failed to create context");
    let test_user = ctx.data_factory
        .create_active_user("retrieveuser", "retrieve@example.com")
        .await
        .expect("Failed to create test user");

    // Test successful user retrieval by ID via HTTP
    let (status, response) = client
        .get(&format!("/users/{}", test_user.id))
        .await
        .expect("Failed to make get user request");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["id"].as_str().unwrap(), test_user.id.to_string());
    assert_eq!(response["username"].as_str().unwrap(), "retrieveuser");
    assert_eq!(response["email"].as_str().unwrap(), "retrieve@example.com");

    // Test user not found
    let non_existent_id = Uuid::new_v4();
    let (status, response) = client
        .get(&format!("/users/{}", non_existent_id))
        .await
        .expect("Failed to make get user request");

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(response.get("error").is_some() || response.get("message").is_some());
}

/// Test authentication middleware
#[tokio::test]
async fn test_authentication_middleware() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test accessing protected endpoint without authentication
    let (status, _response) = client
        .get("/profile")
        .await
        .expect("Failed to make protected request");

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// Test invalid JSON payload handling
#[tokio::test]
async fn test_invalid_json_payload() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test with malformed JSON
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/register")
        .header("content-type", "application/json")
        .body(Body::from("{invalid json}"))
        .expect("Failed to build request");

    let response = client.router.clone().oneshot(request).await.expect("Failed to make request");
    let status = response.status();

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Test content type validation
#[tokio::test]
async fn test_content_type_validation() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test with wrong content type
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/register")
        .header("content-type", "text/plain")
        .body(Body::from(r#"{"username": "test"}"#))
        .expect("Failed to build request");

    let response = client.router.clone().oneshot(request).await.expect("Failed to make request");
    let status = response.status();

    // Should reject non-JSON content type for JSON endpoints
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

/// Test API versioning endpoints
#[tokio::test]
async fn test_api_versioning() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test different API versions
    let versions = vec!["v1", "v2", "v3"];

    for version in versions {
        let (status, _response) = client
            .get(&format!("/api/{}", version))
            .await
            .expect("Failed to make versioned request");

        // Should return some response (200 for supported, possibly different for unsupported)
        assert!(status.is_success() || status.is_client_error());
    }

    // Test unsupported version
    let (status, _response) = client
        .get("/api/v99")
        .await
        .expect("Failed to make unsupported version request");

    // Should handle unsupported versions appropriately
    assert!(status.is_client_error());
}

/// Test password reset flow
#[tokio::test]
async fn test_password_reset_flow() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Create a test user first
    let ctx = IntegrationTestContext::new().await.expect("Failed to create context");
    let _user = ctx.data_factory
        .create_active_user("resetuser", "reset@example.com")
        .await
        .expect("Failed to create test user");

    // Test password reset request
    let reset_request = json!({
        "email": "reset@example.com"
    });

    let (status, response) = client
        .post("/api/request-password-reset", reset_request)
        .await
        .expect("Failed to make password reset request");

    assert_eq!(status, StatusCode::OK);
    assert!(response["message"].as_str().unwrap().contains("password reset"));
}

/// Test pagination endpoints
#[tokio::test]
async fn test_pagination_endpoints() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test pagination endpoint
    let (status, response) = client
        .get("/api/users/paginated?page=1&limit=10")
        .await
        .expect("Failed to make pagination request");

    assert_eq!(status, StatusCode::OK);

    // Response should have pagination structure
    assert!(response.get("data").is_some() ||
            response.get("users").is_some() ||
            response.get("results").is_some());
}

/// Test bulk operations require authentication
#[tokio::test]
async fn test_bulk_operations_require_auth() {
    let client = ApiTestClient::new().await.expect("Failed to create test client");

    // Test bulk operations without authentication
    let bulk_payload = json!({
        "users": [
            {"username": "bulk1", "email": "bulk1@example.com", "password": "pass123"},
            {"username": "bulk2", "email": "bulk2@example.com", "password": "pass123"}
        ]
    });

    let (status, _response) = client
        .post("/api/bulk/users/create", bulk_payload)
        .await
        .expect("Failed to make bulk request");

    // Should require authentication
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}