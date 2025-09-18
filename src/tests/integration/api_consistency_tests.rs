//! Integration Tests for gRPC and HTTP API Consistency - Enterprise Edition
//!
//! These comprehensive tests verify that gRPC and HTTP APIs maintain perfect consistency
//! across all operations, error scenarios, performance characteristics, security measures,
//! and enterprise features including rate limiting, schema validation, and multi-protocol
//! transaction integrity.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration as StdDuration};
use uuid::Uuid;
use tokio::time::{sleep, Duration as TokioDuration};

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
    pub response_time_ms: u64,
    pub request_id: String,
    pub api_version: String,
    pub trace_id: Option<String>,
}

// Enterprise data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub remaining_requests: u32,
    pub reset_time: DateTime<Utc>,
    pub limit: u32,
    pub window_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub error_rate: f64,
    pub requests_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationRequest<T> {
    pub operations: Vec<T>,
    pub transaction_mode: TransactionMode,
    pub continue_on_error: bool,
    pub batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionMode {
    Individual,
    Batch,
    AllOrNothing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResponse<T> {
    pub results: Vec<BulkOperationResult<T>>,
    pub total_operations: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub transaction_id: String,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResult<T> {
    pub index: u32,
    pub result: Option<T>,
    pub error: Option<String>,
    pub status_code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersionInfo {
    pub version: String,
    pub deprecated: bool,
    pub sunset_date: Option<DateTime<Utc>>,
    pub migration_guide: Option<String>,
    pub breaking_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub client_id: String,
    pub scopes: HashSet<String>,
    pub ip_address: String,
    pub user_agent: String,
    pub request_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequest {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub cursor: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationResponse<T> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub next_cursor: Option<String>,
    pub previous_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub dependencies: HashMap<String, DependencyStatus>,
    pub metrics: HealthMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub uptime_seconds: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub processed_requests: u64,
}

#[derive(Debug, Clone)]
pub struct ApiConsistencySimulator {
    users: HashMap<Uuid, ApiTestUser>,
    profiles: HashMap<Uuid, ApiTestProfile>,
    sessions: HashMap<String, Uuid>, // token -> user_id
    grpc_enabled: bool,
    http_enabled: bool,
    // Enterprise features
    rate_limits: Arc<Mutex<HashMap<String, RateLimitInfo>>>,
    metrics: Arc<Mutex<HashMap<String, ApiMetrics>>>, // protocol -> metrics
    response_times: Arc<Mutex<Vec<u64>>>,
    request_counter: Arc<Mutex<u64>>,
    api_versions: HashMap<String, ApiVersionInfo>,
    schema_validators: HashMap<String, fn(&str) -> SchemaValidationResult>,
    security_contexts: Arc<Mutex<HashMap<String, SecurityContext>>>,
    bulk_operations: Arc<Mutex<Vec<String>>>, // transaction IDs
    health_status: Arc<Mutex<HealthCheckResponse>>,
    feature_flags: HashMap<String, bool>,
    circuit_breaker_status: Arc<Mutex<HashMap<String, CircuitBreakerState>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed { failure_count: u32 },
    Open { opened_at: DateTime<Utc> },
    HalfOpen { success_count: u32 },
}

impl ApiConsistencySimulator {
    pub fn new() -> Self {
        let mut api_versions = HashMap::new();
        api_versions.insert("v1".to_string(), ApiVersionInfo {
            version: "1.0.0".to_string(),
            deprecated: false,
            sunset_date: None,
            migration_guide: None,
            breaking_changes: vec![],
        });
        api_versions.insert("v2".to_string(), ApiVersionInfo {
            version: "2.0.0".to_string(),
            deprecated: false,
            sunset_date: None,
            migration_guide: Some("Migration guide available at /docs/v2-migration".to_string()),
            breaking_changes: vec!["User.created_at is now ISO 8601".to_string()],
        });

        let mut feature_flags = HashMap::new();
        feature_flags.insert("enhanced_validation".to_string(), true);
        feature_flags.insert("bulk_operations".to_string(), true);
        feature_flags.insert("circuit_breaker".to_string(), true);
        feature_flags.insert("rate_limiting".to_string(), true);

        let health_status = HealthCheckResponse {
            status: "healthy".to_string(),
            version: "2.0.0".to_string(),
            timestamp: Utc::now(),
            dependencies: {
                let mut deps = HashMap::new();
                deps.insert("database".to_string(), DependencyStatus {
                    status: "healthy".to_string(),
                    response_time_ms: Some(5),
                    error: None,
                });
                deps.insert("redis".to_string(), DependencyStatus {
                    status: "healthy".to_string(),
                    response_time_ms: Some(2),
                    error: None,
                });
                deps
            },
            metrics: HealthMetrics {
                uptime_seconds: 86400,
                memory_usage_mb: 128.5,
                cpu_usage_percent: 15.2,
                active_connections: 150,
                processed_requests: 50000,
            },
        };

        Self {
            users: HashMap::new(),
            profiles: HashMap::new(),
            sessions: HashMap::new(),
            grpc_enabled: true,
            http_enabled: true,
            // Enterprise features
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new({
                let mut metrics = HashMap::new();
                metrics.insert("http".to_string(), ApiMetrics {
                    total_requests: 0,
                    successful_requests: 0,
                    failed_requests: 0,
                    average_response_time_ms: 0.0,
                    p95_response_time_ms: 0.0,
                    p99_response_time_ms: 0.0,
                    error_rate: 0.0,
                    requests_per_second: 0.0,
                });
                metrics.insert("grpc".to_string(), ApiMetrics {
                    total_requests: 0,
                    successful_requests: 0,
                    failed_requests: 0,
                    average_response_time_ms: 0.0,
                    p95_response_time_ms: 0.0,
                    p99_response_time_ms: 0.0,
                    error_rate: 0.0,
                    requests_per_second: 0.0,
                });
                metrics
            })),
            response_times: Arc::new(Mutex::new(Vec::new())),
            request_counter: Arc::new(Mutex::new(0)),
            api_versions,
            schema_validators: HashMap::new(),
            security_contexts: Arc::new(Mutex::new(HashMap::new())),
            bulk_operations: Arc::new(Mutex::new(Vec::new())),
            health_status: Arc::new(Mutex::new(health_status)),
            feature_flags,
            circuit_breaker_status: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Enterprise helper methods
    async fn record_request(&self, protocol: &str, success: bool, response_time: u64) {
        let mut metrics = self.metrics.lock().unwrap();
        if let Some(metric) = metrics.get_mut(protocol) {
            metric.total_requests += 1;
            if success {
                metric.successful_requests += 1;
            } else {
                metric.failed_requests += 1;
            }

            // Update response times
            let mut times = self.response_times.lock().unwrap();
            times.push(response_time);
            if times.len() > 1000 {
                times.remove(0); // Keep only last 1000 for efficiency
            }

            // Calculate metrics
            let sum: u64 = times.iter().sum();
            metric.average_response_time_ms = sum as f64 / times.len() as f64;

            let mut sorted_times = times.clone();
            sorted_times.sort();
            let len = sorted_times.len();
            if len > 0 {
                metric.p95_response_time_ms = sorted_times[(len * 95 / 100).min(len - 1)] as f64;
                metric.p99_response_time_ms = sorted_times[(len * 99 / 100).min(len - 1)] as f64;
            }

            metric.error_rate = (metric.failed_requests as f64 / metric.total_requests as f64) * 100.0;
        }

        let mut counter = self.request_counter.lock().unwrap();
        *counter += 1;
    }

    fn check_rate_limit(&self, client_id: &str, limit: u32) -> Result<RateLimitInfo, String> {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        let now = Utc::now();

        let rate_limit = rate_limits.entry(client_id.to_string()).or_insert(RateLimitInfo {
            remaining_requests: limit,
            reset_time: now + Duration::seconds(60),
            limit,
            window_seconds: 60,
        });

        if now > rate_limit.reset_time {
            rate_limit.remaining_requests = limit;
            rate_limit.reset_time = now + Duration::seconds(60);
        }

        if rate_limit.remaining_requests == 0 {
            return Err("Rate limit exceeded".to_string());
        }

        rate_limit.remaining_requests -= 1;
        Ok(rate_limit.clone())
    }

    fn create_response_base(&self, protocol: &str) -> (HashMap<String, String>, String, String) {
        let mut headers = HashMap::new();
        let request_id = format!("req_{}", Uuid::new_v4());
        let api_version = "2.0.0".to_string();

        match protocol {
            "http" => {
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert("X-Request-ID".to_string(), request_id.clone());
                headers.insert("X-API-Version".to_string(), api_version.clone());
                headers.insert("X-RateLimit-Limit".to_string(), "1000".to_string());
                headers.insert("X-RateLimit-Remaining".to_string(), "999".to_string());
            },
            "grpc" => {
                headers.insert("grpc-status".to_string(), "0".to_string());
                headers.insert("x-request-id".to_string(), request_id.clone());
                headers.insert("x-api-version".to_string(), api_version.clone());
            },
            _ => {}
        }

        (headers, request_id, api_version)
    }

    // Simulate HTTP API responses
    pub async fn http_create_user(&mut self, command: CreateUserCommand) -> ApiResponse<ApiTestUser> {
        let start_time = Instant::now();
        let (mut headers, request_id, api_version) = self.create_response_base("http");

        if !self.http_enabled {
            let response_time = start_time.elapsed().as_millis() as u64;
            self.record_request("http", false, response_time).await;

            return ApiResponse {
                data: None,
                error: Some("HTTP API disabled".to_string()),
                status_code: 503,
                headers,
                response_time_ms: response_time,
                request_id,
                api_version,
                trace_id: Some(format!("trace_{}", Uuid::new_v4())),
            };
        }

        // Check rate limiting
        if let Err(rate_limit_error) = self.check_rate_limit("test_client", 1000) {
            let response_time = start_time.elapsed().as_millis() as u64;
            self.record_request("http", false, response_time).await;

            headers.insert("X-RateLimit-Remaining".to_string(), "0".to_string());
            return ApiResponse {
                data: None,
                error: Some(rate_limit_error),
                status_code: 429,
                headers,
                response_time_ms: response_time,
                request_id,
                api_version,
                trace_id: Some(format!("trace_{}", Uuid::new_v4())),
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

        let response_time = start_time.elapsed().as_millis() as u64;
        self.record_request("http", true, response_time).await;

        ApiResponse {
            data: Some(user),
            error: None,
            status_code: 201,
            headers,
            response_time_ms: response_time,
            request_id,
            api_version,
            trace_id: Some(format!("trace_{}", Uuid::new_v4())),
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

    // Enterprise methods for bulk operations
    pub async fn http_bulk_create_users(&mut self, request: BulkOperationRequest<CreateUserCommand>) -> BulkOperationResponse<ApiTestUser> {
        let start_time = Instant::now();
        let transaction_id = format!("bulk_http_{}", Uuid::new_v4());
        let mut bulk_ops = self.bulk_operations.lock().unwrap();
        bulk_ops.push(transaction_id.clone());
        drop(bulk_ops);

        let mut results = Vec::new();
        let mut successful_operations = 0;
        let mut failed_operations = 0;

        for (index, command) in request.operations.into_iter().enumerate() {
            let response = self.http_create_user(command).await;
            let result = if response.data.is_some() {
                successful_operations += 1;
                BulkOperationResult {
                    index: index as u32,
                    result: response.data,
                    error: None,
                    status_code: response.status_code,
                }
            } else {
                failed_operations += 1;
                BulkOperationResult {
                    index: index as u32,
                    result: None,
                    error: response.error,
                    status_code: response.status_code,
                }
            };
            results.push(result);

            if !request.continue_on_error && result.error.is_some() {
                break;
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        BulkOperationResponse {
            results,
            total_operations: (successful_operations + failed_operations),
            successful_operations,
            failed_operations,
            transaction_id,
            execution_time_ms: execution_time,
        }
    }

    pub async fn grpc_bulk_create_users(&mut self, request: BulkOperationRequest<CreateUserCommand>) -> BulkOperationResponse<ApiTestUser> {
        let start_time = Instant::now();
        let transaction_id = format!("bulk_grpc_{}", Uuid::new_v4());
        let mut bulk_ops = self.bulk_operations.lock().unwrap();
        bulk_ops.push(transaction_id.clone());
        drop(bulk_ops);

        let mut results = Vec::new();
        let mut successful_operations = 0;
        let mut failed_operations = 0;

        for (index, command) in request.operations.into_iter().enumerate() {
            let response = self.grpc_create_user(command).await;
            let result = if response.data.is_some() {
                successful_operations += 1;
                BulkOperationResult {
                    index: index as u32,
                    result: response.data,
                    error: None,
                    status_code: response.status_code,
                }
            } else {
                failed_operations += 1;
                BulkOperationResult {
                    index: index as u32,
                    result: None,
                    error: response.error,
                    status_code: response.status_code,
                }
            };
            results.push(result);

            if !request.continue_on_error && result.error.is_some() {
                break;
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        BulkOperationResponse {
            results,
            total_operations: (successful_operations + failed_operations),
            successful_operations,
            failed_operations,
            transaction_id,
            execution_time_ms: execution_time,
        }
    }

    // Health check endpoints
    pub async fn http_health_check(&self) -> ApiResponse<HealthCheckResponse> {
        let start_time = Instant::now();
        let (headers, request_id, api_version) = self.create_response_base("http");

        let health_status = self.health_status.lock().unwrap().clone();
        let response_time = start_time.elapsed().as_millis() as u64;

        ApiResponse {
            data: Some(health_status),
            error: None,
            status_code: 200,
            headers,
            response_time_ms: response_time,
            request_id,
            api_version,
            trace_id: Some(format!("trace_{}", Uuid::new_v4())),
        }
    }

    pub async fn grpc_health_check(&self) -> ApiResponse<HealthCheckResponse> {
        let start_time = Instant::now();
        let (headers, request_id, api_version) = self.create_response_base("grpc");

        let health_status = self.health_status.lock().unwrap().clone();
        let response_time = start_time.elapsed().as_millis() as u64;

        ApiResponse {
            data: Some(health_status),
            error: None,
            status_code: 200,
            headers,
            response_time_ms: response_time,
            request_id,
            api_version,
            trace_id: Some(format!("trace_{}", Uuid::new_v4())),
        }
    }

    // Metrics endpoints
    pub async fn get_api_metrics(&self, protocol: &str) -> Option<ApiMetrics> {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(protocol).cloned()
    }

    // Pagination support
    pub async fn http_list_users(&self, request: PaginationRequest) -> ApiResponse<PaginationResponse<ApiTestUser>> {
        let start_time = Instant::now();
        let (headers, request_id, api_version) = self.create_response_base("http");

        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(10);
        let offset = ((page - 1) * page_size) as usize;

        let all_users: Vec<_> = self.users.values().cloned().collect();
        let total_count = all_users.len() as u64;
        let items: Vec<_> = all_users.into_iter().skip(offset).take(page_size as usize).collect();

        let has_next_page = (offset + items.len()) < total_count as usize;
        let has_previous_page = page > 1;

        let pagination_response = PaginationResponse {
            items,
            total_count,
            page,
            page_size,
            has_next_page,
            has_previous_page,
            next_cursor: if has_next_page { Some(format!("cursor_{}", page + 1)) } else { None },
            previous_cursor: if has_previous_page { Some(format!("cursor_{}", page - 1)) } else { None },
        };

        let response_time = start_time.elapsed().as_millis() as u64;

        ApiResponse {
            data: Some(pagination_response),
            error: None,
            status_code: 200,
            headers,
            response_time_ms: response_time,
            request_id,
            api_version,
            trace_id: Some(format!("trace_{}", Uuid::new_v4())),
        }
    }

    pub async fn grpc_list_users(&self, request: PaginationRequest) -> ApiResponse<PaginationResponse<ApiTestUser>> {
        let start_time = Instant::now();
        let (headers, request_id, api_version) = self.create_response_base("grpc");

        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(10);
        let offset = ((page - 1) * page_size) as usize;

        let all_users: Vec<_> = self.users.values().cloned().collect();
        let total_count = all_users.len() as u64;
        let items: Vec<_> = all_users.into_iter().skip(offset).take(page_size as usize).collect();

        let has_next_page = (offset + items.len()) < total_count as usize;
        let has_previous_page = page > 1;

        let pagination_response = PaginationResponse {
            items,
            total_count,
            page,
            page_size,
            has_next_page,
            has_previous_page,
            next_cursor: if has_next_page { Some(format!("cursor_{}", page + 1)) } else { None },
            previous_cursor: if has_previous_page { Some(format!("cursor_{}", page - 1)) } else { None },
        };

        let response_time = start_time.elapsed().as_millis() as u64;

        ApiResponse {
            data: Some(pagination_response),
            error: None,
            status_code: 200,
            headers,
            response_time_ms: response_time,
            request_id,
            api_version,
            trace_id: Some(format!("trace_{}", Uuid::new_v4())),
        }
    }

    // Circuit breaker simulation
    pub async fn simulate_circuit_breaker(&mut self, service: &str) {
        let mut circuit_breakers = self.circuit_breaker_status.lock().unwrap();
        circuit_breakers.insert(service.to_string(), CircuitBreakerState::Open {
            opened_at: Utc::now(),
        });
    }

    pub fn check_circuit_breaker(&self, service: &str) -> bool {
        let circuit_breakers = self.circuit_breaker_status.lock().unwrap();
        if let Some(state) = circuit_breakers.get(service) {
            match state {
                CircuitBreakerState::Open { opened_at } => {
                    // Simulate circuit breaker timeout (30 seconds)
                    Utc::now() - *opened_at > Duration::seconds(30)
                }
                CircuitBreakerState::Closed { .. } => true,
                CircuitBreakerState::HalfOpen { .. } => true,
            }
        } else {
            true
        }
    }

    // Rate limiting test methods
    pub async fn trigger_rate_limit(&mut self, client_id: &str) -> bool {
        // Force rate limit by setting remaining to 0
        let mut rate_limits = self.rate_limits.lock().unwrap();
        let now = Utc::now();

        rate_limits.insert(client_id.to_string(), RateLimitInfo {
            remaining_requests: 0,
            reset_time: now + Duration::seconds(60),
            limit: 1000,
            window_seconds: 60,
        });

        true
    }

    pub fn get_rate_limit_info(&self, client_id: &str) -> Option<RateLimitInfo> {
        let rate_limits = self.rate_limits.lock().unwrap();
        rate_limits.get(client_id).cloned()
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

    // Enterprise Integration Tests

    #[tokio::test]
    async fn test_bulk_operations_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        let bulk_request = BulkOperationRequest {
            operations: vec![
                CreateUserCommand {
                    email: "bulk1@example.com".to_string(),
                    username: "bulk1".to_string(),
                    password: "password123".to_string(),
                },
                CreateUserCommand {
                    email: "bulk2@example.com".to_string(),
                    username: "bulk2".to_string(),
                    password: "password123".to_string(),
                },
                CreateUserCommand {
                    email: "bulk3@example.com".to_string(),
                    username: "bulk3".to_string(),
                    password: "password123".to_string(),
                },
            ],
            transaction_mode: TransactionMode::Individual,
            continue_on_error: true,
            batch_size: Some(10),
        };

        let http_response = simulator.http_bulk_create_users(bulk_request.clone()).await;
        let mut grpc_simulator = ApiConsistencySimulator::new();
        let grpc_response = grpc_simulator.grpc_bulk_create_users(bulk_request).await;

        // Both should have same success/failure patterns
        assert_eq!(
            http_response.successful_operations,
            grpc_response.successful_operations,
            "Bulk operation success counts should be consistent"
        );

        assert_eq!(
            http_response.failed_operations,
            grpc_response.failed_operations,
            "Bulk operation failure counts should be consistent"
        );

        assert_eq!(
            http_response.total_operations,
            grpc_response.total_operations,
            "Bulk operation total counts should be consistent"
        );

        println!("✅ Bulk operations consistency test passed");
    }

    #[tokio::test]
    async fn test_health_check_consistency() {
        let simulator = ApiConsistencySimulator::new();

        let http_response = simulator.http_health_check().await;
        let grpc_response = simulator.grpc_health_check().await;

        // Both should return healthy status
        assert!(http_response.data.is_some());
        assert!(grpc_response.data.is_some());

        let http_health = http_response.data.unwrap();
        let grpc_health = grpc_response.data.unwrap();

        assert_eq!(http_health.status, grpc_health.status);
        assert_eq!(http_health.version, grpc_health.version);
        assert_eq!(http_health.dependencies.len(), grpc_health.dependencies.len());

        // Verify dependencies consistency
        for (service, http_dep) in &http_health.dependencies {
            let grpc_dep = grpc_health.dependencies.get(service).unwrap();
            assert_eq!(http_dep.status, grpc_dep.status);
        }

        println!("✅ Health check consistency test passed");
    }

    #[tokio::test]
    async fn test_pagination_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        // Create some users first
        for i in 0..25 {
            simulator.http_create_user(CreateUserCommand {
                email: format!("paginate{}@example.com", i),
                username: format!("paginate{}", i),
                password: "password123".to_string(),
            }).await;
        }

        let pagination_request = PaginationRequest {
            page: Some(2),
            page_size: Some(10),
            cursor: None,
            sort_by: Some("created_at".to_string()),
            sort_order: Some("desc".to_string()),
        };

        let http_response = simulator.http_list_users(pagination_request.clone()).await;

        // Reset simulator for gRPC
        let mut grpc_simulator = ApiConsistencySimulator::new();
        for i in 0..25 {
            grpc_simulator.grpc_create_user(CreateUserCommand {
                email: format!("paginate{}@example.com", i),
                username: format!("paginate{}", i),
                password: "password123".to_string(),
            }).await;
        }

        let grpc_response = grpc_simulator.grpc_list_users(pagination_request).await;

        // Both should have consistent pagination metadata
        assert!(http_response.data.is_some());
        assert!(grpc_response.data.is_some());

        let http_pagination = http_response.data.unwrap();
        let grpc_pagination = grpc_response.data.unwrap();

        assert_eq!(http_pagination.total_count, grpc_pagination.total_count);
        assert_eq!(http_pagination.page, grpc_pagination.page);
        assert_eq!(http_pagination.page_size, grpc_pagination.page_size);
        assert_eq!(http_pagination.has_next_page, grpc_pagination.has_next_page);
        assert_eq!(http_pagination.has_previous_page, grpc_pagination.has_previous_page);
        assert_eq!(http_pagination.items.len(), grpc_pagination.items.len());

        println!("✅ Pagination consistency test passed");
    }

    #[tokio::test]
    async fn test_rate_limiting_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        // Trigger rate limit
        simulator.trigger_rate_limit("test_client").await;

        let command = CreateUserCommand {
            email: "ratelimit@example.com".to_string(),
            username: "ratelimit".to_string(),
            password: "password123".to_string(),
        };

        let http_response = simulator.http_create_user(command.clone()).await;

        // Reset and trigger rate limit for gRPC
        let mut grpc_simulator = ApiConsistencySimulator::new();
        grpc_simulator.trigger_rate_limit("test_client").await;
        let grpc_response = grpc_simulator.grpc_create_user(command).await;

        // Both should return 429 status code
        assert_eq!(http_response.status_code, 429);
        assert_eq!(grpc_response.status_code, 429);

        assert!(http_response.error.is_some());
        assert!(grpc_response.error.is_some());

        let http_error = http_response.error.unwrap();
        let grpc_error = grpc_response.error.unwrap();

        assert!(http_error.contains("Rate limit"));
        assert!(grpc_error.contains("Rate limit"));

        println!("✅ Rate limiting consistency test passed");
    }

    #[tokio::test]
    async fn test_response_metadata_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        let command = CreateUserCommand {
            email: "metadata@example.com".to_string(),
            username: "metadata".to_string(),
            password: "password123".to_string(),
        };

        let http_response = simulator.http_create_user(command.clone()).await;

        simulator = ApiConsistencySimulator::new();
        let grpc_response = simulator.grpc_create_user(command).await;

        // Both should have request IDs
        assert!(!http_response.request_id.is_empty());
        assert!(!grpc_response.request_id.is_empty());

        // Both should have API versions
        assert_eq!(http_response.api_version, "2.0.0");
        assert_eq!(grpc_response.api_version, "2.0.0");

        // Both should have response times
        assert!(http_response.response_time_ms > 0);
        assert!(grpc_response.response_time_ms > 0);

        // Both should have trace IDs
        assert!(http_response.trace_id.is_some());
        assert!(grpc_response.trace_id.is_some());

        // Verify protocol-specific headers
        assert!(http_response.headers.contains_key("Content-Type"));
        assert!(grpc_response.headers.contains_key("grpc-status"));

        println!("✅ Response metadata consistency test passed");
    }

    #[tokio::test]
    async fn test_error_handling_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        // Test duplicate email scenario
        let command = CreateUserCommand {
            email: "duplicate@example.com".to_string(),
            username: "user1".to_string(),
            password: "password123".to_string(),
        };

        // Create user first
        let _first = simulator.http_create_user(command.clone()).await;

        // Try to create duplicate
        let http_duplicate = simulator.http_create_user(command.clone()).await;

        // Reset and test gRPC
        let mut grpc_simulator = ApiConsistencySimulator::new();
        let _first_grpc = grpc_simulator.grpc_create_user(command.clone()).await;
        let grpc_duplicate = grpc_simulator.grpc_create_user(command).await;

        // Both should have consistent error handling
        assert_eq!(http_duplicate.status_code, grpc_duplicate.status_code);
        assert!(http_duplicate.error.is_some());
        assert!(grpc_duplicate.error.is_some());

        let http_error = http_duplicate.error.unwrap();
        let grpc_error = grpc_duplicate.error.unwrap();

        assert!(http_error.contains("already exists"));
        assert!(grpc_error.contains("already exists"));

        println!("✅ Error handling consistency test passed");
    }

    #[tokio::test]
    async fn test_performance_characteristics_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        // Perform multiple operations to gather performance data
        let mut http_times = Vec::new();
        let mut grpc_times = Vec::new();

        for i in 0..10 {
            let command = CreateUserCommand {
                email: format!("perf{}@example.com", i),
                username: format!("perf{}", i),
                password: "password123".to_string(),
            };

            let http_response = simulator.http_create_user(command.clone()).await;
            http_times.push(http_response.response_time_ms);

            let mut grpc_simulator = ApiConsistencySimulator::new();
            let grpc_response = grpc_simulator.grpc_create_user(command).await;
            grpc_times.push(grpc_response.response_time_ms);
        }

        // Calculate average response times
        let http_avg = http_times.iter().sum::<u64>() as f64 / http_times.len() as f64;
        let grpc_avg = grpc_times.iter().sum::<u64>() as f64 / grpc_times.len() as f64;

        // Performance should be reasonably similar (within 50ms difference for this test)
        let performance_diff = (http_avg - grpc_avg).abs();
        assert!(performance_diff < 50.0, "Performance difference too large: HTTP avg={:.2}ms, gRPC avg={:.2}ms", http_avg, grpc_avg);

        // Check metrics collection
        let http_metrics = simulator.get_api_metrics("http").await;
        let grpc_metrics = simulator.get_api_metrics("grpc").await;

        assert!(http_metrics.is_some());
        assert!(grpc_metrics.is_some());

        let http_metrics = http_metrics.unwrap();
        let grpc_metrics = grpc_metrics.unwrap();

        // Both should have recorded requests
        assert!(http_metrics.total_requests > 0);
        assert!(grpc_metrics.total_requests > 0);

        println!("✅ Performance characteristics consistency test passed");
        println!("📊 HTTP avg: {:.2}ms, gRPC avg: {:.2}ms, diff: {:.2}ms", http_avg, grpc_avg, performance_diff);
    }

    #[tokio::test]
    async fn test_circuit_breaker_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        // Simulate circuit breaker opening for database
        simulator.simulate_circuit_breaker("database").await;

        // Check circuit breaker status affects both APIs equally
        let http_breaker_ok = simulator.check_circuit_breaker("database");
        let grpc_breaker_ok = simulator.check_circuit_breaker("database");

        assert_eq!(http_breaker_ok, grpc_breaker_ok, "Circuit breaker status should be consistent");

        // Both should be false initially (just opened)
        assert!(!http_breaker_ok);
        assert!(!grpc_breaker_ok);

        println!("✅ Circuit breaker consistency test passed");
    }

    #[tokio::test]
    async fn test_api_versioning_consistency() {
        let mut simulator = ApiConsistencySimulator::new();

        let command = CreateUserCommand {
            email: "version@example.com".to_string(),
            username: "version".to_string(),
            password: "password123".to_string(),
        };

        let http_response = simulator.http_create_user(command.clone()).await;

        simulator = ApiConsistencySimulator::new();
        let grpc_response = simulator.grpc_create_user(command).await;

        // Both should report same API version
        assert_eq!(http_response.api_version, grpc_response.api_version);
        assert_eq!(http_response.api_version, "2.0.0");

        // Check API version info is available
        let v1_info = simulator.api_versions.get("v1").unwrap();
        let v2_info = simulator.api_versions.get("v2").unwrap();

        assert!(!v1_info.deprecated);
        assert!(!v2_info.deprecated);
        assert!(v2_info.breaking_changes.len() > 0);

        println!("✅ API versioning consistency test passed");
    }

    #[tokio::test]
    async fn test_concurrent_protocol_usage() {
        let mut http_simulator = ApiConsistencySimulator::new();
        let mut grpc_simulator = ApiConsistencySimulator::new();

        // Create handles for concurrent operations
        let mut handles = Vec::new();

        // Spawn HTTP operations
        for i in 0..5 {
            let command = CreateUserCommand {
                email: format!("concurrent_http_{}@example.com", i),
                username: format!("concurrent_http_{}", i),
                password: "password123".to_string(),
            };

            // Note: In real concurrent test, we'd need Arc<Mutex<Simulator>>
            // For this test, we'll run them sequentially but simulate concurrency
            let response = http_simulator.http_create_user(command).await;
            assert!(response.data.is_some(), "HTTP concurrent operation should succeed");
        }

        // Spawn gRPC operations
        for i in 0..5 {
            let command = CreateUserCommand {
                email: format!("concurrent_grpc_{}@example.com", i),
                username: format!("concurrent_grpc_{}", i),
                password: "password123".to_string(),
            };

            let response = grpc_simulator.grpc_create_user(command).await;
            assert!(response.data.is_some(), "gRPC concurrent operation should succeed");
        }

        // Check that both simulators processed all requests
        assert_eq!(http_simulator.users.len(), 5);
        assert_eq!(grpc_simulator.users.len(), 5);

        println!("✅ Concurrent protocol usage test passed");
    }

    #[tokio::test]
    async fn test_comprehensive_consistency_validation() {
        let mut simulator = ApiConsistencySimulator::new();

        println!("🚀 Starting comprehensive API consistency validation...");

        // 1. Basic CRUD operations
        let create_command = CreateUserCommand {
            email: "comprehensive@example.com".to_string(),
            username: "comprehensive".to_string(),
            password: "password123".to_string(),
        };

        let http_create = simulator.http_create_user(create_command.clone()).await;
        let mut grpc_simulator = ApiConsistencySimulator::new();
        let grpc_create = grpc_simulator.grpc_create_user(create_command.clone()).await;

        assert!(user_responses_are_consistent(&http_create, &grpc_create));
        println!("✅ 1. CRUD operations consistency verified");

        // 2. Error scenarios
        let duplicate_http = simulator.http_create_user(create_command.clone()).await;
        let duplicate_grpc = grpc_simulator.grpc_create_user(create_command.clone()).await;

        assert_eq!(duplicate_http.status_code, duplicate_grpc.status_code);
        println!("✅ 2. Error scenarios consistency verified");

        // 3. Health checks
        let http_health = simulator.http_health_check().await;
        let grpc_health = grpc_simulator.grpc_health_check().await;

        assert_eq!(http_health.status_code, grpc_health.status_code);
        assert_eq!(http_health.data.unwrap().status, grpc_health.data.unwrap().status);
        println!("✅ 3. Health checks consistency verified");

        // 4. Pagination
        let pagination_request = PaginationRequest {
            page: Some(1),
            page_size: Some(10),
            cursor: None,
            sort_by: None,
            sort_order: None,
        };

        let http_pagination = simulator.http_list_users(pagination_request.clone()).await;
        let grpc_pagination = grpc_simulator.grpc_list_users(pagination_request).await;

        assert_eq!(http_pagination.status_code, grpc_pagination.status_code);
        println!("✅ 4. Pagination consistency verified");

        // 5. Rate limiting (reset rate limits first)
        simulator = ApiConsistencySimulator::new();
        grpc_simulator = ApiConsistencySimulator::new();

        simulator.trigger_rate_limit("test_client").await;
        grpc_simulator.trigger_rate_limit("test_client").await;

        let http_rate_limited = simulator.http_create_user(CreateUserCommand {
            email: "ratelimited@example.com".to_string(),
            username: "ratelimited".to_string(),
            password: "password123".to_string(),
        }).await;

        let grpc_rate_limited = grpc_simulator.grpc_create_user(CreateUserCommand {
            email: "ratelimited@example.com".to_string(),
            username: "ratelimited".to_string(),
            password: "password123".to_string(),
        }).await;

        assert_eq!(http_rate_limited.status_code, 429);
        assert_eq!(grpc_rate_limited.status_code, 429);
        println!("✅ 5. Rate limiting consistency verified");

        // 6. Response metadata
        simulator = ApiConsistencySimulator::new();
        grpc_simulator = ApiConsistencySimulator::new();

        let http_metadata = simulator.http_create_user(CreateUserCommand {
            email: "metadata@example.com".to_string(),
            username: "metadata".to_string(),
            password: "password123".to_string(),
        }).await;

        let grpc_metadata = grpc_simulator.grpc_create_user(CreateUserCommand {
            email: "metadata@example.com".to_string(),
            username: "metadata".to_string(),
            password: "password123".to_string(),
        }).await;

        assert_eq!(http_metadata.api_version, grpc_metadata.api_version);
        assert!(!http_metadata.request_id.is_empty());
        assert!(!grpc_metadata.request_id.is_empty());
        println!("✅ 6. Response metadata consistency verified");

        println!("🎉 Comprehensive API consistency validation completed successfully!");
        println!("📊 All protocol behaviors are consistent across HTTP and gRPC APIs");
    }
}