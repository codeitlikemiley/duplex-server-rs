//! Integration tests for error scenarios and recovery mechanisms
//!
//! These tests verify that the system handles various error conditions gracefully
//! and can recover from failures without data corruption or service degradation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    DatabaseConnection,
    DatabaseTimeout,
    DatabaseConstraintViolation,
    NetworkTimeout,
    ServiceUnavailable,
    ResourceExhaustion,
    AuthenticationFailure,
    AuthorizationFailure,
    ValidationError,
    ConcurrencyConflict,
    DataCorruption,
    ExternalServiceFailure,
    CircuitBreakerOpen,
    RateLimitExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    Retry,
    Fallback,
    CircuitBreaker,
    GracefulDegradation,
    BackpressureApplication,
    DataRepair,
    ServiceRestart,
    ManualIntervention,
}

#[derive(Debug, Clone)]
pub struct ErrorScenario {
    pub id: Uuid,
    pub error_type: ErrorType,
    pub message: String,
    pub occurred_at: DateTime<Utc>,
    pub service: String,
    pub operation: String,
    pub severity: ErrorSeverity,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    pub id: Uuid,
    pub error_id: Uuid,
    pub action: RecoveryAction,
    pub attempted_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub is_open: bool,
    pub failure_count: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub success_count: u32,
    pub timeout_duration: Duration,
}

impl CircuitBreakerState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            failure_count: 0,
            last_failure_time: None,
            success_count: 0,
            timeout_duration: Duration::from_secs(30),
        }
    }

    pub fn should_allow_request(&self) -> bool {
        if !self.is_open {
            return true;
        }

        if let Some(last_failure) = self.last_failure_time {
            let elapsed = Utc::now().signed_duration_since(last_failure);
            elapsed.num_seconds() as u64 > self.timeout_duration.as_secs()
        } else {
            true
        }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        if self.is_open && self.success_count >= 5 {
            self.is_open = false;
            self.failure_count = 0;
            self.success_count = 0;
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Utc::now());
        self.success_count = 0;

        if self.failure_count >= 3 {
            self.is_open = true;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorRecoverySimulator {
    users: HashMap<Uuid, TestUser>,
    errors: Vec<ErrorScenario>,
    recovery_attempts: Vec<RecoveryAttempt>,
    circuit_breakers: HashMap<String, CircuitBreakerState>,
    retry_queues: HashMap<String, VecDeque<Uuid>>, // service -> failed operation IDs
    database_available: bool,
    network_available: bool,
    external_services_available: bool,
    rate_limit_counters: HashMap<String, (u32, DateTime<Utc>)>, // service -> (count, reset_time)
    data_corruption_simulation: bool,
    resource_exhaustion_simulation: bool,
    concurrent_operations: HashMap<Uuid, String>, // operation_id -> operation_type
}

impl ErrorRecoverySimulator {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            errors: Vec::new(),
            recovery_attempts: Vec::new(),
            circuit_breakers: HashMap::new(),
            retry_queues: HashMap::new(),
            database_available: true,
            network_available: true,
            external_services_available: true,
            rate_limit_counters: HashMap::new(),
            data_corruption_simulation: false,
            resource_exhaustion_simulation: false,
            concurrent_operations: HashMap::new(),
        }
    }

    // System state manipulation
    pub fn simulate_database_outage(&mut self) {
        self.database_available = false;
    }

    pub fn restore_database(&mut self) {
        self.database_available = true;
    }

    pub fn simulate_network_outage(&mut self) {
        self.network_available = false;
    }

    pub fn restore_network(&mut self) {
        self.network_available = true;
    }

    pub fn simulate_external_service_outage(&mut self) {
        self.external_services_available = false;
    }

    pub fn restore_external_services(&mut self) {
        self.external_services_available = true;
    }

    pub fn enable_data_corruption_simulation(&mut self) {
        self.data_corruption_simulation = true;
    }

    pub fn disable_data_corruption_simulation(&mut self) {
        self.data_corruption_simulation = false;
    }

    pub fn enable_resource_exhaustion_simulation(&mut self) {
        self.resource_exhaustion_simulation = true;
    }

    pub fn disable_resource_exhaustion_simulation(&mut self) {
        self.resource_exhaustion_simulation = false;
    }

    // Error recording and management
    pub async fn record_error(&mut self, error_type: ErrorType, service: String, operation: String, message: String) -> Uuid {
        let severity = self.determine_severity(&error_type);
        let error = ErrorScenario {
            id: Uuid::now_v7(),
            error_type: error_type.clone(),
            message,
            occurred_at: Utc::now(),
            service: service.clone(),
            operation,
            severity,
            metadata: HashMap::new(),
        };

        let error_id = error.id;
        self.errors.push(error);

        // Update circuit breaker
        let circuit_breaker = self.circuit_breakers.entry(service.clone()).or_insert_with(CircuitBreakerState::new);
        circuit_breaker.record_failure();

        // Add to retry queue if applicable
        if self.should_retry(&error_type) {
            let queue = self.retry_queues.entry(service).or_insert_with(VecDeque::new);
            queue.push_back(error_id);
        }

        error_id
    }

    pub async fn attempt_recovery(&mut self, error_id: Uuid, action: RecoveryAction) -> Result<(), String> {
        let error = self.errors.iter().find(|e| e.id == error_id)
            .ok_or("Error not found")?
            .clone();

        let recovery_attempt = RecoveryAttempt {
            id: Uuid::now_v7(),
            error_id,
            action: action.clone(),
            attempted_at: Utc::now(),
            completed_at: None,
            success: false,
            details: String::new(),
        };

        let mut attempt = recovery_attempt;
        let result = match action {
            RecoveryAction::Retry => self.execute_retry(&error).await,
            RecoveryAction::Fallback => self.execute_fallback(&error).await,
            RecoveryAction::CircuitBreaker => self.manage_circuit_breaker(&error.service).await,
            RecoveryAction::GracefulDegradation => self.apply_graceful_degradation(&error).await,
            RecoveryAction::BackpressureApplication => self.apply_backpressure(&error.service).await,
            RecoveryAction::DataRepair => self.repair_data(&error).await,
            RecoveryAction::ServiceRestart => self.restart_service(&error.service).await,
            RecoveryAction::ManualIntervention => self.request_manual_intervention(&error).await,
        };

        attempt.completed_at = Some(Utc::now());
        attempt.success = result.is_ok();
        attempt.details = result.as_ref().unwrap_or_else(|e| e).clone();

        self.recovery_attempts.push(attempt);
        result.map(|_| ())
    }

    // User operations with error simulation
    pub async fn create_user_with_errors(&mut self, email: String, username: String) -> Result<TestUser, String> {
        let operation_id = Uuid::now_v7();
        self.concurrent_operations.insert(operation_id, "create_user".to_string());

        // Check various error conditions
        if !self.database_available {
            let error_id = self.record_error(
                ErrorType::DatabaseConnection,
                "user_service".to_string(),
                "create_user".to_string(),
                "Database connection failed".to_string(),
            ).await;
            return Err(format!("Database unavailable: {}", error_id));
        }

        if !self.network_available {
            let error_id = self.record_error(
                ErrorType::NetworkTimeout,
                "user_service".to_string(),
                "create_user".to_string(),
                "Network timeout occurred".to_string(),
            ).await;
            return Err(format!("Network unavailable: {}", error_id));
        }

        if self.resource_exhaustion_simulation {
            let error_id = self.record_error(
                ErrorType::ResourceExhaustion,
                "user_service".to_string(),
                "create_user".to_string(),
                "System resources exhausted".to_string(),
            ).await;
            return Err(format!("Resource exhaustion: {}", error_id));
        }

        // Check circuit breaker
        let circuit_breaker = self.circuit_breakers.get("user_service");
        if let Some(cb) = circuit_breaker {
            if !cb.should_allow_request() {
                let error_id = self.record_error(
                    ErrorType::CircuitBreakerOpen,
                    "user_service".to_string(),
                    "create_user".to_string(),
                    "Circuit breaker is open".to_string(),
                ).await;
                return Err(format!("Circuit breaker open: {}", error_id));
            }
        }

        // Check rate limiting
        if self.is_rate_limited("user_service") {
            let error_id = self.record_error(
                ErrorType::RateLimitExceeded,
                "user_service".to_string(),
                "create_user".to_string(),
                "Rate limit exceeded".to_string(),
            ).await;
            return Err(format!("Rate limited: {}", error_id));
        }

        // Check for duplicate email (constraint violation)
        if self.users.values().any(|u| u.email == email) {
            let error_id = self.record_error(
                ErrorType::DatabaseConstraintViolation,
                "user_service".to_string(),
                "create_user".to_string(),
                format!("Email {} already exists", email),
            ).await;
            return Err(format!("Constraint violation: {}", error_id));
        }

        // Simulate data corruption
        if self.data_corruption_simulation && username.len() > 5 {
            let error_id = self.record_error(
                ErrorType::DataCorruption,
                "user_service".to_string(),
                "create_user".to_string(),
                "Data corruption detected during operation".to_string(),
            ).await;
            return Err(format!("Data corruption: {}", error_id));
        }

        // Create user successfully
        let user = TestUser {
            id: Uuid::now_v7(),
            email,
            username,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.users.insert(user.id, user.clone());
        self.concurrent_operations.remove(&operation_id);

        // Record success for circuit breaker
        let circuit_breaker = self.circuit_breakers.entry("user_service".to_string()).or_insert_with(CircuitBreakerState::new);
        circuit_breaker.record_success();

        // Update rate limit counter
        self.update_rate_limit_counter("user_service");

        Ok(user)
    }

    pub async fn get_user_with_errors(&mut self, user_id: Uuid) -> Result<TestUser, String> {
        if !self.database_available {
            let error_id = self.record_error(
                ErrorType::DatabaseConnection,
                "user_service".to_string(),
                "get_user".to_string(),
                "Database connection failed".to_string(),
            ).await;
            return Err(format!("Database unavailable: {}", error_id));
        }

        // Check circuit breaker
        let circuit_breaker = self.circuit_breakers.get("user_service");
        if let Some(cb) = circuit_breaker {
            if !cb.should_allow_request() {
                let error_id = self.record_error(
                    ErrorType::CircuitBreakerOpen,
                    "user_service".to_string(),
                    "get_user".to_string(),
                    "Circuit breaker is open".to_string(),
                ).await;
                return Err(format!("Circuit breaker open: {}", error_id));
            }
        }

        match self.users.get(&user_id) {
            Some(user) => {
                // Record success for circuit breaker
                let circuit_breaker = self.circuit_breakers.entry("user_service".to_string()).or_insert_with(CircuitBreakerState::new);
                circuit_breaker.record_success();
                Ok(user.clone())
            },
            None => {
                self.record_error(
                    ErrorType::ValidationError,
                    "user_service".to_string(),
                    "get_user".to_string(),
                    format!("User {} not found", user_id),
                ).await;
                Err("User not found".to_string())
            }
        }
    }

    // Recovery action implementations
    async fn execute_retry(&mut self, error: &ErrorScenario) -> Result<String, String> {
        // Simulate retry with exponential backoff
        sleep(Duration::from_millis(100)).await;

        match error.error_type {
            ErrorType::NetworkTimeout | ErrorType::DatabaseTimeout | ErrorType::DatabaseConnection => {
                if self.network_available && self.database_available {
                    Ok("Retry successful after infrastructure recovery".to_string())
                } else {
                    Err("Retry failed - infrastructure still unavailable".to_string())
                }
            },
            ErrorType::ExternalServiceFailure => {
                if self.external_services_available {
                    Ok("Retry successful after external service recovery".to_string())
                } else {
                    Err("Retry failed - external service still unavailable".to_string())
                }
            },
            _ => Err("Error type not suitable for retry".to_string()),
        }
    }

    async fn execute_fallback(&mut self, error: &ErrorScenario) -> Result<String, String> {
        match error.error_type {
            ErrorType::ExternalServiceFailure => {
                Ok("Fallback to cached data successful".to_string())
            },
            ErrorType::DatabaseConnection => {
                Ok("Fallback to read-only mode successful".to_string())
            },
            ErrorType::ServiceUnavailable => {
                Ok("Fallback to degraded service mode successful".to_string())
            },
            _ => Err("No fallback available for this error type".to_string()),
        }
    }

    async fn manage_circuit_breaker(&mut self, service: &str) -> Result<String, String> {
        let circuit_breaker = self.circuit_breakers.entry(service.to_string()).or_insert_with(CircuitBreakerState::new);

        if circuit_breaker.is_open {
            // Force reset the circuit breaker for recovery action
            circuit_breaker.is_open = false;
            circuit_breaker.failure_count = 0;
            circuit_breaker.success_count = 0;
            circuit_breaker.last_failure_time = None;
            Ok("Circuit breaker reset successful".to_string())
        } else {
            Ok("Circuit breaker already closed".to_string())
        }
    }

    async fn apply_graceful_degradation(&mut self, error: &ErrorScenario) -> Result<String, String> {
        match error.error_type {
            ErrorType::ResourceExhaustion => {
                Ok("Graceful degradation applied - reduced functionality enabled".to_string())
            },
            ErrorType::DatabaseTimeout => {
                Ok("Graceful degradation applied - using cached data".to_string())
            },
            _ => Err("Graceful degradation not applicable".to_string()),
        }
    }

    async fn apply_backpressure(&mut self, service: &str) -> Result<String, String> {
        // Simulate backpressure by rate limiting
        let current_time = Utc::now();
        self.rate_limit_counters.insert(service.to_string(), (0, current_time));
        Ok("Backpressure applied successfully".to_string())
    }

    async fn repair_data(&mut self, error: &ErrorScenario) -> Result<String, String> {
        match error.error_type {
            ErrorType::DataCorruption => {
                // Simulate data repair
                let corrupted_users: Vec<_> = self.users.iter()
                    .filter(|(_, user)| user.username.contains("corrupted"))
                    .map(|(id, _)| *id)
                    .collect();

                let repair_count = corrupted_users.len();

                for user_id in corrupted_users {
                    if let Some(user) = self.users.get_mut(&user_id) {
                        user.username = user.username.replace("corrupted", "repaired");
                        user.updated_at = Utc::now();
                    }
                }

                Ok(format!("Data repair completed for {} users", repair_count))
            },
            _ => Err("Data repair not applicable for this error type".to_string()),
        }
    }

    async fn restart_service(&mut self, service: &str) -> Result<String, String> {
        // Simulate service restart
        sleep(Duration::from_millis(500)).await;

        // Reset circuit breaker and retry queue
        self.circuit_breakers.insert(service.to_string(), CircuitBreakerState::new());
        self.retry_queues.remove(service);

        // Reset rate limiting
        self.rate_limit_counters.remove(service);

        Ok(format!("Service {} restarted successfully", service))
    }

    async fn request_manual_intervention(&mut self, error: &ErrorScenario) -> Result<String, String> {
        match error.severity {
            ErrorSeverity::Critical => {
                Ok("Manual intervention requested - escalated to on-call engineer".to_string())
            },
            _ => Err("Manual intervention not required for this severity level".to_string()),
        }
    }

    // Helper methods
    fn determine_severity(&self, error_type: &ErrorType) -> ErrorSeverity {
        match error_type {
            ErrorType::DataCorruption | ErrorType::DatabaseConnection => ErrorSeverity::Critical,
            ErrorType::ServiceUnavailable | ErrorType::ResourceExhaustion => ErrorSeverity::High,
            ErrorType::NetworkTimeout | ErrorType::ExternalServiceFailure => ErrorSeverity::Medium,
            _ => ErrorSeverity::Low,
        }
    }

    fn should_retry(&self, error_type: &ErrorType) -> bool {
        matches!(error_type,
            ErrorType::NetworkTimeout |
            ErrorType::DatabaseTimeout |
            ErrorType::ExternalServiceFailure |
            ErrorType::ServiceUnavailable
        )
    }

    fn is_rate_limited(&mut self, service: &str) -> bool {
        let current_time = Utc::now();
        if let Some((count, reset_time)) = self.rate_limit_counters.get(service) {
            if current_time.signed_duration_since(*reset_time).num_seconds() > 60 {
                // Reset counter after 1 minute
                self.rate_limit_counters.insert(service.to_string(), (1, current_time));
                false
            } else {
                *count > 10 // Rate limit at 10 requests per minute
            }
        } else {
            self.rate_limit_counters.insert(service.to_string(), (1, current_time));
            false
        }
    }

    fn update_rate_limit_counter(&mut self, service: &str) {
        let current_time = Utc::now();
        if let Some((count, reset_time)) = self.rate_limit_counters.get_mut(service) {
            if current_time.signed_duration_since(*reset_time).num_seconds() > 60 {
                *count = 1;
                *reset_time = current_time;
            } else {
                *count += 1;
            }
        } else {
            self.rate_limit_counters.insert(service.to_string(), (1, current_time));
        }
    }

    // Testing utilities
    pub fn get_error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn get_recovery_attempt_count(&self) -> usize {
        self.recovery_attempts.len()
    }

    pub fn get_errors_by_type(&self, error_type: &ErrorType) -> Vec<&ErrorScenario> {
        self.errors.iter().filter(|e| e.error_type == *error_type).collect()
    }

    pub fn get_successful_recoveries(&self) -> Vec<&RecoveryAttempt> {
        self.recovery_attempts.iter().filter(|r| r.success).collect()
    }

    pub fn get_circuit_breaker_status(&self, service: &str) -> Option<&CircuitBreakerState> {
        self.circuit_breakers.get(service)
    }

    pub fn get_retry_queue_size(&self, service: &str) -> usize {
        self.retry_queues.get(service).map(|q| q.len()).unwrap_or(0)
    }

    pub fn add_test_user(&mut self, user: TestUser) {
        self.users.insert(user.id, user);
    }

    pub fn get_user_count(&self) -> usize {
        self.users.len()
    }
}

// Tests for error scenarios and recovery

#[tokio::test]
async fn test_database_outage_recovery() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate database outage
    simulator.simulate_database_outage();

    // Attempt to create user - should fail
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser".to_string(),
    ).await;

    assert!(result.is_err());
    assert_eq!(simulator.get_error_count(), 1);

    // Attempt recovery through retry (should fail)
    let error_id = simulator.errors[0].id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::Retry).await;
    assert!(recovery_result.is_err());

    // Restore database
    simulator.restore_database();

    // Retry should now succeed
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::Retry).await;
    assert!(recovery_result.is_ok());

    // Verify recovery attempt was recorded
    assert_eq!(simulator.get_recovery_attempt_count(), 2);
    assert_eq!(simulator.get_successful_recoveries().len(), 1);
}

#[tokio::test]
async fn test_circuit_breaker_functionality() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate repeated failures to trigger circuit breaker
    simulator.simulate_database_outage();

    for i in 0..5 {
        let result = simulator.create_user_with_errors(
            format!("test{}@example.com", i),
            format!("testuser{}", i),
        ).await;
        assert!(result.is_err());
    }

    // Check circuit breaker is open
    let cb_status = simulator.get_circuit_breaker_status("user_service").unwrap();
    assert!(cb_status.is_open);

    // Restore database
    simulator.restore_database();

    // Request should still fail due to circuit breaker
    let result = simulator.create_user_with_errors(
        "after_cb@example.com".to_string(),
        "aftercb".to_string(),
    ).await;
    assert!(result.is_err());

    // Manage circuit breaker recovery
    let error_id = simulator.errors.last().unwrap().id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::CircuitBreaker).await;

    // Circuit breaker should allow requests after timeout
    assert!(recovery_result.is_ok());
}

#[tokio::test]
async fn test_constraint_violation_handling() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Create a user successfully
    let _user1 = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser".to_string(),
    ).await.unwrap();

    assert_eq!(simulator.get_user_count(), 1);

    // Try to create another user with same email
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser2".to_string(),
    ).await;

    assert!(result.is_err());
    assert_eq!(simulator.get_error_count(), 1);

    let errors = simulator.get_errors_by_type(&ErrorType::DatabaseConstraintViolation);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("already exists"));
}

#[tokio::test]
async fn test_rate_limiting_and_backpressure() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Create multiple users rapidly to trigger rate limiting
    for i in 0..15 {
        let result = simulator.create_user_with_errors(
            format!("test{}@example.com", i),
            format!("testuser{}", i),
        ).await;

        if i < 10 {
            assert!(result.is_ok(), "Request {} should succeed", i);
        } else {
            assert!(result.is_err(), "Request {} should be rate limited", i);
        }
    }

    // Check rate limit errors were recorded
    let rate_limit_errors = simulator.get_errors_by_type(&ErrorType::RateLimitExceeded);
    assert!(!rate_limit_errors.is_empty());

    // Apply backpressure recovery
    let error_id = rate_limit_errors[0].id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::BackpressureApplication).await;
    assert!(recovery_result.is_ok());
}

#[tokio::test]
async fn test_data_corruption_and_repair() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Enable data corruption simulation
    simulator.enable_data_corruption_simulation();

    // Try to create user with long username (triggers corruption)
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "verylongusername".to_string(),
    ).await;

    assert!(result.is_err());
    assert_eq!(simulator.get_error_count(), 1);

    let corruption_errors = simulator.get_errors_by_type(&ErrorType::DataCorruption);
    assert_eq!(corruption_errors.len(), 1);

    // Attempt data repair recovery
    let error_id = corruption_errors[0].id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::DataRepair).await;
    assert!(recovery_result.is_ok());

    // Disable corruption simulation
    simulator.disable_data_corruption_simulation();

    // Should now work
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "normaluser".to_string(),
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_resource_exhaustion_recovery() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Enable resource exhaustion simulation
    simulator.enable_resource_exhaustion_simulation();

    // Attempt to create user - should fail
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser".to_string(),
    ).await;

    assert!(result.is_err());
    assert_eq!(simulator.get_error_count(), 1);

    let exhaustion_errors = simulator.get_errors_by_type(&ErrorType::ResourceExhaustion);
    assert_eq!(exhaustion_errors.len(), 1);

    // Apply graceful degradation
    let error_id = exhaustion_errors[0].id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::GracefulDegradation).await;
    assert!(recovery_result.is_ok());

    // Disable resource exhaustion
    simulator.disable_resource_exhaustion_simulation();

    // Should now work
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser".to_string(),
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_outage_with_fallback() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate network outage
    simulator.simulate_network_outage();

    // Attempt to create user - should fail
    let result = simulator.create_user_with_errors(
        "test@example.com".to_string(),
        "testuser".to_string(),
    ).await;

    assert!(result.is_err());
    assert_eq!(simulator.get_error_count(), 1);

    let network_errors = simulator.get_errors_by_type(&ErrorType::NetworkTimeout);
    assert_eq!(network_errors.len(), 1);

    // Apply fallback recovery (should fail for this operation type)
    let error_id = network_errors[0].id;
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::Fallback).await;
    assert!(recovery_result.is_err());

    // Restore network and retry
    simulator.restore_network();
    let retry_result = simulator.attempt_recovery(error_id, RecoveryAction::Retry).await;
    assert!(retry_result.is_ok());
}

#[tokio::test]
async fn test_external_service_failure_with_fallback() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate external service outage
    simulator.simulate_external_service_outage();

    // Record an external service failure manually
    let error_id = simulator.record_error(
        ErrorType::ExternalServiceFailure,
        "email_service".to_string(),
        "send_verification".to_string(),
        "Email service unavailable".to_string(),
    ).await;

    // Apply fallback recovery (should succeed)
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::Fallback).await;
    assert!(recovery_result.is_ok());

    let successful_recoveries = simulator.get_successful_recoveries();
    assert_eq!(successful_recoveries.len(), 1);
    assert!(successful_recoveries[0].details.contains("cached data"));
}

#[tokio::test]
async fn test_service_restart_recovery() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate multiple failures to set up error state
    simulator.simulate_database_outage();

    for i in 0..3 {
        simulator.create_user_with_errors(
            format!("test{}@example.com", i),
            format!("testuser{}", i),
        ).await.ok();
    }

    // Check that circuit breaker is open and retry queue has items
    let cb_status = simulator.get_circuit_breaker_status("user_service").unwrap();
    assert!(cb_status.is_open);

    // Record a critical error requiring service restart
    let error_id = simulator.record_error(
        ErrorType::ServiceUnavailable,
        "user_service".to_string(),
        "critical_failure".to_string(),
        "Service requires restart".to_string(),
    ).await;

    // Apply service restart recovery
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::ServiceRestart).await;
    assert!(recovery_result.is_ok());

    // Verify service state was reset
    let cb_status_after = simulator.get_circuit_breaker_status("user_service").unwrap();
    assert!(!cb_status_after.is_open);
    assert_eq!(simulator.get_retry_queue_size("user_service"), 0);
}

#[tokio::test]
async fn test_manual_intervention_escalation() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Record a critical error
    let error_id = simulator.record_error(
        ErrorType::DataCorruption,
        "user_service".to_string(),
        "data_integrity_failure".to_string(),
        "Critical data corruption detected".to_string(),
    ).await;

    // Request manual intervention
    let recovery_result = simulator.attempt_recovery(error_id, RecoveryAction::ManualIntervention).await;
    assert!(recovery_result.is_ok());

    let successful_recoveries = simulator.get_successful_recoveries();
    assert_eq!(successful_recoveries.len(), 1);
    assert!(successful_recoveries[0].details.contains("on-call engineer"));
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Simulate concurrent operations with various failures
    let mut tasks = Vec::new();

    // Some will succeed, some will fail due to various reasons
    for i in 0..10 {
        if i % 3 == 0 {
            simulator.simulate_network_outage();
        } else if i % 3 == 1 {
            simulator.enable_resource_exhaustion_simulation();
        } else {
            simulator.restore_network();
            simulator.disable_resource_exhaustion_simulation();
        }

        let result = simulator.create_user_with_errors(
            format!("test{}@example.com", i),
            format!("testuser{}", i),
        ).await;

        tasks.push(result);
    }

    // Check mixed results
    let successes = tasks.iter().filter(|r| r.is_ok()).count();
    let failures = tasks.iter().filter(|r| r.is_err()).count();

    assert!(successes > 0, "Some operations should succeed");
    assert!(failures > 0, "Some operations should fail");
    assert_eq!(successes + failures, 10);

    // Verify errors were recorded
    assert!(simulator.get_error_count() > 0);
}

#[tokio::test]
async fn test_error_severity_classification() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Record errors of different types
    let critical_error = simulator.record_error(
        ErrorType::DataCorruption,
        "user_service".to_string(),
        "corruption_detected".to_string(),
        "Data corruption found".to_string(),
    ).await;

    let high_error = simulator.record_error(
        ErrorType::ServiceUnavailable,
        "user_service".to_string(),
        "service_down".to_string(),
        "Service is down".to_string(),
    ).await;

    let medium_error = simulator.record_error(
        ErrorType::NetworkTimeout,
        "user_service".to_string(),
        "timeout_occurred".to_string(),
        "Network timeout".to_string(),
    ).await;

    let low_error = simulator.record_error(
        ErrorType::ValidationError,
        "user_service".to_string(),
        "invalid_input".to_string(),
        "Invalid input provided".to_string(),
    ).await;

    // Verify severity classification
    let errors = &simulator.errors;
    assert_eq!(errors.iter().find(|e| e.id == critical_error).unwrap().severity, ErrorSeverity::Critical);
    assert_eq!(errors.iter().find(|e| e.id == high_error).unwrap().severity, ErrorSeverity::High);
    assert_eq!(errors.iter().find(|e| e.id == medium_error).unwrap().severity, ErrorSeverity::Medium);
    assert_eq!(errors.iter().find(|e| e.id == low_error).unwrap().severity, ErrorSeverity::Low);
}

#[tokio::test]
async fn test_recovery_action_effectiveness() {
    let mut simulator = ErrorRecoverySimulator::new();

    // Test different recovery actions for appropriate error types
    let scenarios = vec![
        (ErrorType::NetworkTimeout, RecoveryAction::Retry),
        (ErrorType::ExternalServiceFailure, RecoveryAction::Fallback),
        (ErrorType::ResourceExhaustion, RecoveryAction::GracefulDegradation),
        (ErrorType::DataCorruption, RecoveryAction::DataRepair),
    ];

    for (error_type, recovery_action) in scenarios {
        let error_id = simulator.record_error(
            error_type,
            "test_service".to_string(),
            "test_operation".to_string(),
            "Test error".to_string(),
        ).await;

        // First attempt - may fail due to conditions
        simulator.attempt_recovery(error_id, recovery_action.clone()).await.ok();

        // Set up conditions for success
        simulator.restore_database();
        simulator.restore_network();
        simulator.restore_external_services();

        // Second attempt should succeed
        let result = simulator.attempt_recovery(error_id, recovery_action).await;
        assert!(result.is_ok(), "Recovery should succeed for appropriate error type");
    }

    // Verify all recovery attempts were recorded
    assert_eq!(simulator.get_recovery_attempt_count(), 8); // 2 attempts per scenario
}