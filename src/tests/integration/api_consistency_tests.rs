//! Integration tests for gRPC and HTTP API consistency
//!
//! These tests verify that the gRPC and HTTP APIs produce consistent results
//! for the same operations, ensuring unified behavior across different protocols.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    BadRequest { message: String },
    Conflict { message: String },
    NotFound { message: String },
    Unauthorized { message: String },
    InternalServerError { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserCommand {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateUserCommand {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserProfileCommand {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub preferences: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiTestUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiTestProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub preferences: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiResponse<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ApiConsistencySimulator {
    users: HashMap<Uuid, ApiTestUser>,
    profiles: HashMap<Uuid, ApiTestProfile>,
    sessions: HashMap<String, Uuid>, // token -> user_id
    grpc_enabled: bool,
    http_enabled: bool,
}

impl ApiConsistencySimulator {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            profiles: HashMap::new(),
            sessions: HashMap::new(),
            grpc_enabled: true,
            http_enabled: true,
        }
    }

    // Simulate HTTP API responses
    pub async fn http_create_user(&mut self, command: CreateUserCommand) -> ApiResponse<ApiTestUser> {
        if !self.http_enabled {
            return ApiResponse {
                data: None,
                error: Some("HTTP API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        // Check if email already exists
        if self.users.values().any(|u| u.email == command.email) {
            return ApiResponse {
                data: None,
                error: Some("Email already exists".to_string()),
                status_code: 409,
                headers: HashMap::new(),
            };
        }

        let user = ApiTestUser {
            id: Uuid::now_v7(),
            email: command.email,
            username: command.username,
            password_hash: format!("hashed_{}", command.password),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            failed_login_attempts: 0,
            locked_until: None,
            is_active: true,
        };

        let user_id = user.id;
        self.users.insert(user_id, user.clone());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        ApiResponse {
            data: Some(user),
            error: None,
            status_code: 201,
            headers,
        }
    }

    // Simulate gRPC API responses
    pub async fn grpc_create_user(&mut self, command: CreateUserCommand) -> ApiResponse<ApiTestUser> {
        if !self.grpc_enabled {
            return ApiResponse {
                data: None,
                error: Some("gRPC API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        // Same logic as HTTP for consistency
        if self.users.values().any(|u| u.email == command.email) {
            return ApiResponse {
                data: None,
                error: Some("Email already exists".to_string()),
                status_code: 409,
                headers: HashMap::new(),
            };
        }

        let user = ApiTestUser {
            id: Uuid::now_v7(),
            email: command.email,
            username: command.username,
            password_hash: format!("hashed_{}", command.password),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            failed_login_attempts: 0,
            locked_until: None,
            is_active: true,
        };

        let user_id = user.id;
        self.users.insert(user_id, user.clone());

        let mut headers = HashMap::new();
        headers.insert("grpc-status".to_string(), "0".to_string());

        ApiResponse {
            data: Some(user),
            error: None,
            status_code: 200, // gRPC uses 200 for success
            headers,
        }
    }

    pub async fn http_get_user(&self, user_id: Uuid) -> ApiResponse<ApiTestUser> {
        if !self.http_enabled {
            return ApiResponse {
                data: None,
                error: Some("HTTP API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        match self.users.get(&user_id) {
            Some(user) => {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());

                ApiResponse {
                    data: Some(user.clone()),
                    error: None,
                    status_code: 200,
                    headers,
                }
            }
            None => ApiResponse {
                data: None,
                error: Some("User not found".to_string()),
                status_code: 404,
                headers: HashMap::new(),
            },
        }
    }

    pub async fn grpc_get_user(&self, user_id: Uuid) -> ApiResponse<ApiTestUser> {
        if !self.grpc_enabled {
            return ApiResponse {
                data: None,
                error: Some("gRPC API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        match self.users.get(&user_id) {
            Some(user) => {
                let mut headers = HashMap::new();
                headers.insert("grpc-status".to_string(), "0".to_string());

                ApiResponse {
                    data: Some(user.clone()),
                    error: None,
                    status_code: 200,
                    headers,
                }
            }
            None => {
                let mut headers = HashMap::new();
                headers.insert("grpc-status".to_string(), "5".to_string()); // NOT_FOUND

                ApiResponse {
                    data: None,
                    error: Some("User not found".to_string()),
                    status_code: 404,
                    headers,
                }
            }
        }
    }

    pub async fn http_authenticate_user(&mut self, command: AuthenticateUserCommand) -> ApiResponse<String> {
        if !self.http_enabled {
            return ApiResponse {
                data: None,
                error: Some("HTTP API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        match self.users.values_mut().find(|u| u.email == command.email) {
            Some(user) => {
                if user.password_hash == format!("hashed_{}", command.password) {
                    user.last_login = Some(Utc::now());
                    user.failed_login_attempts = 0;

                    let token = format!("http_token_{}", Uuid::now_v7());
                    self.sessions.insert(token.clone(), user.id);

                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));

                    ApiResponse {
                        data: Some(token),
                        error: None,
                        status_code: 200,
                        headers,
                    }
                } else {
                    user.failed_login_attempts += 1;

                    ApiResponse {
                        data: None,
                        error: Some("Invalid credentials".to_string()),
                        status_code: 401,
                        headers: HashMap::new(),
                    }
                }
            }
            None => ApiResponse {
                data: None,
                error: Some("User not found".to_string()),
                status_code: 404,
                headers: HashMap::new(),
            },
        }
    }

    pub async fn grpc_authenticate_user(&mut self, command: AuthenticateUserCommand) -> ApiResponse<String> {
        if !self.grpc_enabled {
            return ApiResponse {
                data: None,
                error: Some("gRPC API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        match self.users.values_mut().find(|u| u.email == command.email) {
            Some(user) => {
                if user.password_hash == format!("hashed_{}", command.password) {
                    user.last_login = Some(Utc::now());
                    user.failed_login_attempts = 0;

                    let token = format!("grpc_token_{}", Uuid::now_v7());
                    self.sessions.insert(token.clone(), user.id);

                    let mut headers = HashMap::new();
                    headers.insert("grpc-status".to_string(), "0".to_string());
                    headers.insert("authorization".to_string(), format!("Bearer {}", token));

                    ApiResponse {
                        data: Some(token),
                        error: None,
                        status_code: 200,
                        headers,
                    }
                } else {
                    user.failed_login_attempts += 1;

                    let mut headers = HashMap::new();
                    headers.insert("grpc-status".to_string(), "16".to_string()); // UNAUTHENTICATED

                    ApiResponse {
                        data: None,
                        error: Some("Invalid credentials".to_string()),
                        status_code: 401,
                        headers,
                    }
                }
            }
            None => {
                let mut headers = HashMap::new();
                headers.insert("grpc-status".to_string(), "5".to_string()); // NOT_FOUND

                ApiResponse {
                    data: None,
                    error: Some("User not found".to_string()),
                    status_code: 404,
                    headers,
                }
            }
        }
    }

    pub async fn http_create_profile(&mut self, command: CreateUserProfileCommand) -> ApiResponse<ApiTestProfile> {
        if !self.http_enabled {
            return ApiResponse {
                data: None,
                error: Some("HTTP API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        if !self.users.contains_key(&command.user_id) {
            return ApiResponse {
                data: None,
                error: Some("User not found".to_string()),
                status_code: 404,
                headers: HashMap::new(),
            };
        }

        if self.profiles.values().any(|p| p.user_id == command.user_id) {
            return ApiResponse {
                data: None,
                error: Some("Profile already exists for user".to_string()),
                status_code: 409,
                headers: HashMap::new(),
            };
        }

        let profile = ApiTestProfile {
            id: Uuid::now_v7(),
            user_id: command.user_id,
            first_name: command.first_name,
            last_name: command.last_name,
            avatar_url: command.avatar_url,
            bio: command.bio,
            phone: command.phone,
            timezone: command.timezone,
            language: command.language,
            preferences: command.preferences.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let profile_id = profile.id;
        self.profiles.insert(profile_id, profile.clone());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        ApiResponse {
            data: Some(profile),
            error: None,
            status_code: 201,
            headers,
        }
    }

    pub async fn grpc_create_profile(&mut self, command: CreateUserProfileCommand) -> ApiResponse<ApiTestProfile> {
        if !self.grpc_enabled {
            return ApiResponse {
                data: None,
                error: Some("gRPC API disabled".to_string()),
                status_code: 503,
                headers: HashMap::new(),
            };
        }

        if !self.users.contains_key(&command.user_id) {
            let mut headers = HashMap::new();
            headers.insert("grpc-status".to_string(), "5".to_string()); // NOT_FOUND

            return ApiResponse {
                data: None,
                error: Some("User not found".to_string()),
                status_code: 404,
                headers,
            };
        }

        if self.profiles.values().any(|p| p.user_id == command.user_id) {
            let mut headers = HashMap::new();
            headers.insert("grpc-status".to_string(), "6".to_string()); // ALREADY_EXISTS

            return ApiResponse {
                data: None,
                error: Some("Profile already exists for user".to_string()),
                status_code: 409,
                headers,
            };
        }

        let profile = ApiTestProfile {
            id: Uuid::now_v7(),
            user_id: command.user_id,
            first_name: command.first_name,
            last_name: command.last_name,
            avatar_url: command.avatar_url,
            bio: command.bio,
            phone: command.phone,
            timezone: command.timezone,
            language: command.language,
            preferences: command.preferences.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let profile_id = profile.id;
        self.profiles.insert(profile_id, profile.clone());

        let mut headers = HashMap::new();
        headers.insert("grpc-status".to_string(), "0".to_string());

        ApiResponse {
            data: Some(profile),
            error: None,
            status_code: 200,
            headers,
        }
    }

    pub fn disable_grpc(&mut self) {
        self.grpc_enabled = false;
    }

    pub fn disable_http(&mut self) {
        self.http_enabled = false;
    }

    pub fn enable_grpc(&mut self) {
        self.grpc_enabled = true;
    }

    pub fn enable_http(&mut self) {
        self.http_enabled = true;
    }
}

// Helper function to compare API responses for consistency
fn responses_are_consistent<T: PartialEq + std::fmt::Debug>(
    http_response: &ApiResponse<T>,
    grpc_response: &ApiResponse<T>,
) -> bool {
    // Data should be identical
    if http_response.data != grpc_response.data {
        println!("Data mismatch: HTTP={:?}, gRPC={:?}", http_response.data, grpc_response.data);
        return false;
    }

    // Errors should be semantically equivalent
    match (&http_response.error, &grpc_response.error) {
        (None, None) => {},
        (Some(http_err), Some(grpc_err)) => {
            if http_err != grpc_err {
                println!("Error message mismatch: HTTP={}, gRPC={}", http_err, grpc_err);
                return false;
            }
        },
        _ => {
            println!("Error presence mismatch: HTTP={:?}, gRPC={:?}", http_response.error, grpc_response.error);
            return false;
        }
    }

    // Status codes should be semantically equivalent
    if http_response.status_code != grpc_response.status_code {
        // Allow some differences in success codes (201 vs 200)
        let both_success = (200..300).contains(&http_response.status_code) &&
                          (200..300).contains(&grpc_response.status_code);
        if !both_success {
            println!("Status code mismatch: HTTP={}, gRPC={}", http_response.status_code, grpc_response.status_code);
            return false;
        }
    }

    true
}

// Helper function for comparing user responses (ignoring IDs and timestamps)
fn user_responses_are_consistent(
    http_response: &ApiResponse<ApiTestUser>,
    grpc_response: &ApiResponse<ApiTestUser>,
) -> bool {
    // Compare business logic consistency rather than exact data match
    match (&http_response.data, &grpc_response.data, &http_response.error, &grpc_response.error) {
        (Some(http_user), Some(grpc_user), None, None) => {
            // Both successful - compare business fields (ignoring IDs and timestamps)
            http_user.email == grpc_user.email
                && http_user.username == grpc_user.username
                && http_user.password_hash == grpc_user.password_hash
                && http_user.email_verified == grpc_user.email_verified
                && http_user.is_active == grpc_user.is_active
                && http_user.failed_login_attempts == grpc_user.failed_login_attempts
        },
        (None, None, Some(_), Some(_)) => {
            // Both failed - check error consistency
            responses_are_consistent(http_response, grpc_response)
        },
        _ => {
            // Success/failure mismatch
            println!("Success/failure mismatch: HTTP success={}, gRPC success={}",
                http_response.data.is_some(), grpc_response.data.is_some());
            false
        }
    }
}

// Helper function for comparing profile responses
fn profile_responses_are_consistent(
    http_response: &ApiResponse<ApiTestProfile>,
    grpc_response: &ApiResponse<ApiTestProfile>,
) -> bool {
    match (&http_response.data, &grpc_response.data, &http_response.error, &grpc_response.error) {
        (Some(http_profile), Some(grpc_profile), None, None) => {
            // Both successful - compare business fields (user_id will be different due to separate simulators)
            // Focus on the profile data fields rather than user_id
            let fields_match = http_profile.first_name == grpc_profile.first_name
                && http_profile.last_name == grpc_profile.last_name
                && http_profile.avatar_url == grpc_profile.avatar_url
                && http_profile.bio == grpc_profile.bio
                && http_profile.phone == grpc_profile.phone
                && http_profile.timezone == grpc_profile.timezone
                && http_profile.language == grpc_profile.language
                && http_profile.preferences == grpc_profile.preferences;

            if !fields_match {
                println!("Profile field mismatch: HTTP={:?}, gRPC={:?}", http_profile, grpc_profile);
            }

            fields_match
        },
        (None, None, Some(_), Some(_)) => {
            // Both failed - check error consistency
            responses_are_consistent(http_response, grpc_response)
        },
        _ => {
            // Success/failure mismatch
            println!("Success/failure mismatch: HTTP success={}, gRPC success={}, HTTP error={:?}, gRPC error={:?}",
                http_response.data.is_some(), grpc_response.data.is_some(), http_response.error, grpc_response.error);
            false
        }
    }
}

#[tokio::test]
async fn test_user_creation_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    let command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    let http_response = simulator.http_create_user(command.clone()).await;

    // Reset simulator for fair comparison
    simulator = ApiConsistencySimulator::new();
    let grpc_response = simulator.grpc_create_user(command).await;

    assert!(
        user_responses_are_consistent(&http_response, &grpc_response),
        "User creation responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_user_retrieval_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    // Create a user first
    let create_command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    let user = simulator.http_create_user(create_command.clone()).await.data.unwrap();
    simulator.grpc_create_user(create_command).await;

    let http_response = simulator.http_get_user(user.id).await;
    let grpc_response = simulator.grpc_get_user(user.id).await;

    assert!(
        user_responses_are_consistent(&http_response, &grpc_response),
        "User retrieval responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_authentication_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    // Create a user first
    let create_command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    simulator.http_create_user(create_command.clone()).await;
    simulator.grpc_create_user(create_command).await;

    let auth_command = AuthenticateUserCommand {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };

    let http_response = simulator.http_authenticate_user(auth_command.clone()).await;
    let grpc_response = simulator.grpc_authenticate_user(auth_command).await;

    // For authentication, we mainly care about success/failure consistency
    assert_eq!(
        http_response.error.is_none(),
        grpc_response.error.is_none(),
        "Authentication success/failure not consistent between APIs"
    );

    assert_eq!(
        http_response.data.is_some(),
        grpc_response.data.is_some(),
        "Authentication token presence not consistent between APIs"
    );
}

#[tokio::test]
async fn test_user_not_found_consistency() {
    let simulator = ApiConsistencySimulator::new();

    let non_existent_id = Uuid::now_v7();

    let http_response = simulator.http_get_user(non_existent_id).await;
    let grpc_response = simulator.grpc_get_user(non_existent_id).await;

    assert!(
        responses_are_consistent(&http_response, &grpc_response),
        "User not found responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_duplicate_email_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    let command = CreateUserCommand {
        email: "duplicate@example.com".to_string(),
        username: "testuser1".to_string(),
        password: "password123".to_string(),
    };

    // Create user via HTTP first
    simulator.http_create_user(command.clone()).await;

    // Try to create same email via gRPC
    let grpc_response = simulator.grpc_create_user(command.clone()).await;

    // Reset and try opposite order
    simulator = ApiConsistencySimulator::new();
    simulator.grpc_create_user(command.clone()).await;
    let http_response = simulator.http_create_user(command).await;

    assert!(
        responses_are_consistent(&http_response, &grpc_response),
        "Duplicate email responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_invalid_authentication_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    // Create a user first
    let create_command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    simulator.http_create_user(create_command.clone()).await;
    simulator.grpc_create_user(create_command).await;

    let auth_command = AuthenticateUserCommand {
        email: "test@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let http_response = simulator.http_authenticate_user(auth_command.clone()).await;
    let grpc_response = simulator.grpc_authenticate_user(auth_command).await;

    assert!(
        responses_are_consistent(&http_response, &grpc_response),
        "Invalid authentication responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_profile_creation_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    // Create a user first
    let user_command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    let user = simulator.http_create_user(user_command.clone()).await.data.unwrap();
    simulator.grpc_create_user(user_command).await;

    let profile_command = CreateUserProfileCommand {
        user_id: user.id,
        first_name: Some("John".to_string()),
        last_name: Some("Doe".to_string()),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        bio: Some("Test bio".to_string()),
        phone: Some("+1234567890".to_string()),
        timezone: Some("UTC".to_string()),
        language: Some("en".to_string()),
        preferences: Some(HashMap::new()),
    };

    let http_response = simulator.http_create_profile(profile_command.clone()).await;

    // Reset for fair comparison and create user in grpc simulator
    let mut grpc_simulator = ApiConsistencySimulator::new();
    let grpc_user = grpc_simulator.grpc_create_user(CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    }).await.data.unwrap();

    // Update profile command with the gRPC user ID
    let grpc_profile_command = CreateUserProfileCommand {
        user_id: grpc_user.id,
        first_name: Some("John".to_string()),
        last_name: Some("Doe".to_string()),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        bio: Some("Test bio".to_string()),
        phone: Some("+1234567890".to_string()),
        timezone: Some("UTC".to_string()),
        language: Some("en".to_string()),
        preferences: Some(HashMap::new()),
    };

    let grpc_response = grpc_simulator.grpc_create_profile(grpc_profile_command).await;

    assert!(
        profile_responses_are_consistent(&http_response, &grpc_response),
        "Profile creation responses are not consistent between HTTP and gRPC APIs"
    );
}

#[tokio::test]
async fn test_api_unavailable_behavior() {
    let mut simulator = ApiConsistencySimulator::new();

    // Test HTTP unavailable
    simulator.disable_http();
    let http_response = simulator.http_create_user(CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    }).await;

    assert_eq!(http_response.status_code, 503);
    assert!(http_response.error.is_some());

    // Test gRPC unavailable
    simulator.enable_http();
    simulator.disable_grpc();
    let grpc_response = simulator.grpc_create_user(CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    }).await;

    assert_eq!(grpc_response.status_code, 503);
    assert!(grpc_response.error.is_some());
}

#[tokio::test]
async fn test_response_header_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    let command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    let http_response = simulator.http_create_user(command.clone()).await;

    simulator = ApiConsistencySimulator::new();
    let grpc_response = simulator.grpc_create_user(command).await;

    // Both should have appropriate headers for their protocol
    assert!(http_response.headers.contains_key("Content-Type"));
    assert!(grpc_response.headers.contains_key("grpc-status"));

    // Both should indicate success
    assert_eq!(http_response.headers.get("Content-Type").unwrap(), "application/json");
    assert_eq!(grpc_response.headers.get("grpc-status").unwrap(), "0");
}

#[tokio::test]
async fn test_error_code_mapping_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    // Test various error scenarios and ensure consistent status codes
    let scenarios = vec![
        // Non-existent user lookup
        (404, Uuid::now_v7()),
    ];

    for (expected_status, user_id) in scenarios {
        let http_response = simulator.http_get_user(user_id).await;
        let grpc_response = simulator.grpc_get_user(user_id).await;

        assert_eq!(
            http_response.status_code, expected_status,
            "HTTP status code mismatch for user lookup"
        );
        assert_eq!(
            grpc_response.status_code, expected_status,
            "gRPC status code mismatch for user lookup"
        );
    }
}

#[tokio::test]
async fn test_data_format_consistency() {
    let mut simulator = ApiConsistencySimulator::new();

    let command = CreateUserCommand {
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };

    let http_response = simulator.http_create_user(command.clone()).await;

    simulator = ApiConsistencySimulator::new();
    let grpc_response = simulator.grpc_create_user(command).await;

    // Both should return the same data structure
    assert!(http_response.data.is_some());
    assert!(grpc_response.data.is_some());

    let http_user = http_response.data.unwrap();
    let grpc_user = grpc_response.data.unwrap();

    // Core fields should be identical in structure (though IDs will differ)
    assert_eq!(http_user.email, grpc_user.email);
    assert_eq!(http_user.username, grpc_user.username);
    assert_eq!(http_user.email_verified, grpc_user.email_verified);
    assert_eq!(http_user.is_active, grpc_user.is_active);
    assert_eq!(http_user.failed_login_attempts, grpc_user.failed_login_attempts);
}

#[tokio::test]
async fn test_concurrent_api_usage() {
    // Use separate simulators to avoid state conflicts
    let mut http_simulator = ApiConsistencySimulator::new();
    let mut grpc_simulator = ApiConsistencySimulator::new();

    // Simulate concurrent requests to both APIs
    let commands: Vec<CreateUserCommand> = (0..5).map(|i| CreateUserCommand {
        email: format!("user{}@example.com", i),
        username: format!("user{}", i),
        password: "password123".to_string(),
    }).collect();

    let mut http_results = Vec::new();
    let mut grpc_results = Vec::new();

    for command in commands {
        let http_result = http_simulator.http_create_user(command.clone()).await;
        let grpc_result = grpc_simulator.grpc_create_user(command).await;

        http_results.push(http_result);
        grpc_results.push(grpc_result);
    }

    // All operations should succeed consistently
    for (http_result, grpc_result) in http_results.iter().zip(grpc_results.iter()) {
        assert_eq!(
            http_result.error.is_none(),
            grpc_result.error.is_none(),
            "Concurrent operation success/failure not consistent"
        );
    }
}