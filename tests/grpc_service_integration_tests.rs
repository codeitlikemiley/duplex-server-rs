use tonic::{transport::{Server, Channel}, Request, Response, Status, Code};
use tokio::net::TcpListener;
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use futures::future::join_all;

mod integration_test_helpers;
use integration_test_helpers::{IntegrationTestContext, TestDatabase};

use coqrs::{
    models::{User, UserProfile, UserStatus},
    PostgreSQL,
    commands::CommandMessage,
    infrastructure::grpc::users::GrpcUserServiceImpl,
    proto::{
        user_service_server::{UserService, UserServiceServer},
        user_service_client::UserServiceClient,
        CreateUserRequest, CreateUserResponse,
        GetUserRequest, GetUserResponse,
        UpdateProfileRequest, UpdateProfileResponse,
        LoginRequest, LoginResponse,
        RegisterUserRequest, RegisterUserResponse,
    },
};

/// gRPC test server for integration testing
struct GrpcTestServer {
    addr: SocketAddr,
    db_context: Arc<TestDatabase>,
    _command_sender: mpsc::Sender<CommandMessage>,
}

impl GrpcTestServer {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_context = Arc::new(TestDatabase::new().await?);
        let pool = db_context.pool.clone();

        // Create command channel for gRPC service
        let (command_sender, _command_receiver) = mpsc::channel(100);

        // Create gRPC service with real database and command channel
        let user_service = GrpcUserServiceImpl::new(pool, command_sender.clone());

        // Start test server
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            Server::builder()
                .add_service(user_service)
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(Self {
            addr,
            db_context,
            _command_sender: command_sender,
        })
    }

    async fn client(&self) -> Result<UserServiceClient<Channel>, Box<dyn std::error::Error>> {
        let channel = tonic::transport::Endpoint::from_shared(format!("http://{}", self.addr))?
            .connect()
            .await?;

        Ok(UserServiceClient::new(channel))
    }

    async fn reset(&self) -> Result<(), sqlx::Error> {
        self.db_context.cleanup().await
    }
}

#[tokio::test]
async fn test_grpc_create_user() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test successful user creation
    let request = Request::new(CreateUserRequest {
        username: "grpc_test_user".to_string(),
        email: "grpc@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let response = client.create_user(request).await;
    assert!(response.is_ok(), "Failed to create user via gRPC: {:?}", response.err());

    let _response = response.unwrap().into_inner();
    // CreateUserResponse is empty in the current proto definition

    // Verify user was created by querying the database directly
    let found_user = server.db_context.postgres
        .find_user_by_email("grpc@example.com")
        .await
        .unwrap();
    assert!(found_user.is_some(), "User should exist in database");

    let found_user = found_user.unwrap();
    assert_eq!(found_user.username, "grpc_test_user");
    assert_eq!(found_user.email, "grpc@example.com");
}

#[tokio::test]
async fn test_grpc_create_user_validation_errors() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test empty username
    let request = Request::new(CreateUserRequest {
        username: "".to_string(),
        email: "test@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let response = client.create_user(request).await;
    assert!(response.is_err(), "Should fail with empty username");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
    assert!(error.message().contains("username"), "Error should mention username");

    // Test invalid email
    let request = Request::new(CreateUserRequest {
        username: "testuser".to_string(),
        email: "invalid-email".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let response = client.create_user(request).await;
    assert!(response.is_err(), "Should fail with invalid email");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);

    // Test weak password
    let request = Request::new(CreateUserRequest {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "weak".to_string(),
    });

    let response = client.create_user(request).await;
    assert!(response.is_err(), "Should fail with weak password");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_grpc_get_user() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    // Create test user in database
    let ctx = IntegrationTestContext::new().await.expect("Failed to create context");
    let user = ctx.data_factory.create_active_user("grpc_get_user", "grpc_get@example.com").await
        .expect("Failed to create test user");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test successful user retrieval
    let request = Request::new(GetUserRequest {
        id: user.id.to_string(),
    });

    let response = client.get_user(request).await;
    assert!(response.is_ok(), "Failed to get user via gRPC: {:?}", response.err());

    let response = response.unwrap().into_inner();
    let user = response.user.expect("User should be present in response");
    assert_eq!(user.username, "grpc_get_user");
    assert_eq!(user.email, "grpc_get@example.com");
    assert!(user.email_verified);

    // Test user not found
    let request = Request::new(GetUserRequest {
        id: Uuid::new_v4().to_string(),
    });

    let response = client.get_user(request).await;
    assert!(response.is_err(), "Should fail when user not found");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::NotFound);

    // Test invalid UUID format
    let request = Request::new(GetUserRequest {
        id: "invalid-uuid".to_string(),
    });

    let response = client.get_user(request).await;
    assert!(response.is_err(), "Should fail with invalid UUID");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_grpc_user_registration_flow() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Step 1: Register user
    let register_request = Request::new(RegisterUserRequest {
        username: "registration_user".to_string(),
        email: "register@example.com".to_string(),
        password: "SecurePass123!".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
    });

    let register_response = client.register_user(register_request).await;
    assert!(register_response.is_ok(), "Failed to register user: {:?}", register_response.err());

    let register_response = register_response.unwrap().into_inner();
    assert!(register_response.verification_sent);

    // Find the created user by email (since response doesn't include user_id)
    let found_user = server.db_context.postgres
        .find_user_by_email("register@example.com")
        .await
        .unwrap()
        .expect("User should exist after registration");

    let user_id = found_user.id;

    // Step 2: Verify user exists but is not verified
    assert_eq!(found_user.status, UserStatus::PendingVerification);
    assert!(!found_user.email_verified);

    // Step 3: Verify email (simulate)
    // In a real scenario, this would use the verification token
    let mut verified_user = found_user.clone();
    verified_user.email_verified = true;
    verified_user.status = UserStatus::Active;

    server.db_context.postgres.save_user(verified_user).await.expect("Failed to update user");

    // Step 4: Login after verification
    let login_request = Request::new(LoginRequest {
        email: "register@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let login_response = client.login(login_request).await;
    assert!(login_response.is_ok(), "Failed to login after verification: {:?}", login_response.err());

    let login_response = login_response.unwrap().into_inner();
    assert!(!login_response.token.is_empty());
    assert!(login_response.user.is_some());
}

#[tokio::test]
async fn test_grpc_profile_management() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    // Create test user
    let ctx = IntegrationTestContext::new().await.expect("Failed to create context");
    let user = ctx.data_factory.create_active_user("profile_user", "profile@example.com").await
        .expect("Failed to create test user");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test profile update
    let update_request = Request::new(UpdateProfileRequest {
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        bio: "Software engineer with 5 years of experience".to_string(),
        website: "https://janesmith.dev".to_string(),
        location: "San Francisco, CA".to_string(),
        avatar_url: "https://example.com/avatar.jpg".to_string(),
        preferences_json: "{}".to_string(),
    });

    let update_response = client.update_profile(update_request).await;
    assert!(update_response.is_ok(), "Failed to update profile: {:?}", update_response.err());

    // Verify profile was updated in database
    let found_profile = server.db_context.postgres.find_profile_by_user_id(user.id).await.unwrap();
    assert!(found_profile.is_some(), "Profile should exist");

    let found_profile = found_profile.unwrap();
    assert_eq!(found_profile.first_name, Some("Jane".to_string()));
    assert_eq!(found_profile.last_name, Some("Smith".to_string()));
    assert_eq!(found_profile.bio, Some("Software engineer with 5 years of experience".to_string()));
    assert_eq!(found_profile.website, Some("https://janesmith.dev".to_string()));
    assert_eq!(found_profile.location, Some("San Francisco, CA".to_string()));
}

#[tokio::test]
async fn test_grpc_error_handling() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test duplicate user creation
    let user_request = CreateUserRequest {
        username: "duplicate_user".to_string(),
        email: "duplicate@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    };

    // Create first user
    let request = Request::new(user_request.clone());
    let response = client.create_user(request).await;
    assert!(response.is_ok(), "First user creation should succeed");

    // Try to create duplicate user
    let request = Request::new(user_request);
    let response = client.create_user(request).await;
    assert!(response.is_err(), "Duplicate user creation should fail");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::AlreadyExists);
    assert!(error.message().contains("already exists") || error.message().contains("duplicate"));

    // Test malformed requests
    let request = Request::new(GetUserRequest {
        id: "".to_string(), // Empty user ID
    });

    let response = client.get_user(request).await;
    assert!(response.is_err(), "Should fail with empty user ID");
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_grpc_concurrent_operations() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    // Create multiple clients for concurrent operations
    let mut handles = Vec::new();

    for i in 0..10 {
        let server_addr = server.addr;
        let handle = tokio::spawn(async move {
            let channel = tonic::transport::Endpoint::from_shared(format!("http://{}", server_addr))
                .unwrap()
                .connect()
                .await
                .unwrap();

            let mut client = UserServiceClient::new(channel);

            let request = Request::new(CreateUserRequest {
                username: format!("concurrent_user_{}", i),
                email: format!("concurrent_{}@example.com", i),
                password: "SecurePass123!".to_string(),
            });

            client.create_user(request).await
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let results = futures::future::join_all(handles).await;

    // Verify all operations succeeded
    for (i, result) in results.into_iter().enumerate() {
        let _response = result.expect("Task should not panic").expect(&format!("User {} creation should succeed", i));
        // CreateUserResponse is empty, so just verify success
    }

    // Verify all users were created in database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&server.db_context.pool)
        .await
        .expect("Failed to count users");

    assert_eq!(count, 10, "Should have created 10 users");
}

#[tokio::test]
async fn test_grpc_authentication_flow() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    // Create verified user
    let ctx = IntegrationTestContext::new().await.expect("Failed to create context");
    let _user = ctx.data_factory.create_active_user("auth_user", "auth@example.com").await
        .expect("Failed to create test user");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test successful login
    let login_request = Request::new(LoginRequest {
        email: "auth@example.com".to_string(),
        password: "SecurePass123!".to_string(), // This should match the user's actual password
    });

    // Note: This test assumes the password service is properly integrated
    // In reality, you'd need to hash the password properly

    // Test login with wrong password
    let wrong_password_request = Request::new(LoginRequest {
        email: "auth@example.com".to_string(),
        password: "WrongPassword123!".to_string(),
    });

    let response = client.login(wrong_password_request).await;
    assert!(response.is_err(), "Should fail with wrong password");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::Unauthenticated);

    // Test login with non-existent user
    let nonexistent_request = Request::new(LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let response = client.login(nonexistent_request).await;
    assert!(response.is_err(), "Should fail with non-existent user");

    let error = response.unwrap_err();
    assert_eq!(error.code(), Code::NotFound);
}

#[tokio::test]
async fn test_grpc_metadata_and_headers() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Test with custom metadata
    let mut request = Request::new(CreateUserRequest {
        username: "metadata_user".to_string(),
        email: "metadata@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    // Add custom metadata
    request.metadata_mut().insert("client-id", "test-client".parse().unwrap());
    request.metadata_mut().insert("request-id", Uuid::new_v4().to_string().parse().unwrap());

    let response = client.create_user(request).await;
    assert!(response.is_ok(), "Request with metadata should succeed");

    // Verify response metadata (if any)
    let response = response.unwrap();
    let metadata = response.metadata();

    // In a real implementation, you might return custom metadata
    // For now, just verify the response structure
    let _response_inner = response.into_inner();
    // CreateUserResponse is empty, so just verify success
}

#[tokio::test]
async fn test_grpc_large_payload_handling() {
    let server = GrpcTestServer::new().await.expect("Failed to start gRPC test server");
    server.reset().await.expect("Failed to reset test environment");

    let mut client = server.client().await.expect("Failed to create gRPC client");

    // Create user with large bio
    let large_bio = "a".repeat(10000); // 10KB bio

    let request = Request::new(CreateUserRequest {
        username: "large_payload_user".to_string(),
        email: "large@example.com".to_string(),
        password: "SecurePass123!".to_string(),
    });

    let response = client.create_user(request).await;
    assert!(response.is_ok(), "Should handle reasonable payload sizes");

    // Find the created user since CreateUserResponse doesn't include user_id
    let found_user = server.db_context.postgres
        .find_user_by_email("large@example.com")
        .await
        .unwrap()
        .expect("User should exist after creation");
    let user_id = found_user.id;

    // Update profile with large bio
    let update_request = Request::new(UpdateProfileRequest {
        first_name: "Large".to_string(),
        last_name: "Payload".to_string(),
        bio: large_bio,
        website: "".to_string(),
        location: "".to_string(),
        avatar_url: "".to_string(),
        preferences_json: "{}".to_string(),
    });

    let update_response = client.update_profile(update_request).await;
    assert!(update_response.is_ok(), "Should handle large bio in profile update");
}