//! Integration tests for rate limiting behavior
//! Enhanced with enterprise features for production-ready rate limiting testing

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub status: UserStatus,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum AppError {
    NotFound {
        resource: String,
        id: Option<String>,
    },
    Unauthorized {
        message: String,
    },
    BadRequest {
        message: String,
    },
    TooManyRequests {
        message: String,
        retry_after: Option<u64>,
    },
    InternalServerError {
        message: String,
    },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::TooManyRequests { message, retry_after } => {
                if let Some(retry) = retry_after {
                    write!(f, "{} (retry after {} seconds)", message, retry)
                } else {
                    write!(f, "{}", message)
                }
            },
            AppError::Unauthorized { message } => write!(f, "Unauthorized: {}", message),
            AppError::BadRequest { message } => write!(f, "Bad request: {}", message),
            AppError::NotFound { resource, id } => {
                if let Some(id) = id {
                    write!(f, "{} not found: {}", resource, id)
                } else {
                    write!(f, "{} not found", resource)
                }
            },
            AppError::InternalServerError { message } => write!(f, "Internal error: {}", message),
        }
    }
}

// Enhanced structures for enterprise rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitType {
    Login,
    Registration,
    PasswordReset,
    EmailVerification,
    ApiRequest,
    FileUpload,
    AdminAction,
    BulkOperation,
    SearchQuery,
    DataExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_attempts: u32,
    pub window_duration: Duration,
    pub block_duration: Option<Duration>,
    pub burst_limit: Option<u32>,
    pub exponential_backoff: bool,
    pub whitelist_ips: HashSet<IpAddr>,
    pub priority_users: HashSet<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEntry {
    pub count: u32,
    pub window_start: DateTime<Utc>,
    pub last_request: DateTime<Utc>,
    pub blocked_until: Option<DateTime<Utc>>,
    pub backoff_multiplier: f64,
    pub burst_count: u32,
    pub burst_window_start: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitViolation {
    pub id: Uuid,
    pub identifier: String, // IP or user ID
    pub limit_type: RateLimitType,
    pub violation_time: DateTime<Utc>,
    pub attempts_made: u32,
    pub limit_exceeded: u32,
    pub source_info: SourceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
    pub geographic_location: Option<String>,
    pub isp: Option<String>,
    pub device_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitMetrics {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub violation_count: u64,
    pub avg_request_rate: f64,
    pub peak_request_rate: f64,
    pub top_violators: Vec<(String, u64)>,
    pub block_effectiveness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRateLimit {
    pub base_limit: u32,
    pub current_limit: u32,
    pub adjustment_factor: f64,
    pub load_threshold: f64,
    pub last_adjustment: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedRateLimit {
    pub node_id: String,
    pub global_count: Arc<RwLock<HashMap<String, u32>>>,
    pub sync_interval: Duration,
    pub last_sync: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub timeout_duration: Duration,
    pub last_failure: Option<DateTime<Utc>>,
    pub state: CircuitState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,  // Normal operation
    Open,    // Circuit breaker is open, blocking requests
    HalfOpen, // Testing if service has recovered
}

pub struct RateLimitingBehaviorSimulator {
    users: Vec<User>,
    configs: HashMap<RateLimitType, RateLimitConfig>,
    ip_entries: HashMap<(IpAddr, RateLimitType), RateLimitEntry>,
    user_entries: HashMap<(Uuid, RateLimitType), RateLimitEntry>,
    global_entries: HashMap<RateLimitType, RateLimitEntry>,
    violations: Vec<RateLimitViolation>,
    metrics: RateLimitMetrics,
    adaptive_limits: HashMap<RateLimitType, AdaptiveRateLimit>,
    circuit_breakers: HashMap<String, CircuitBreakerState>,
    distributed_state: Option<DistributedRateLimit>,
    geo_blocking: HashMap<String, bool>, // country_code -> blocked
    suspicious_patterns: HashMap<String, Vec<DateTime<Utc>>>, // pattern -> timestamps
}

impl RateLimitingBehaviorSimulator {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // Enhanced configurations with enterprise features
        configs.insert(RateLimitType::Login, RateLimitConfig {
            max_attempts: 5,
            window_duration: Duration::minutes(15),
            block_duration: Some(Duration::hours(1)),
            burst_limit: Some(10),
            exponential_backoff: true,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        configs.insert(RateLimitType::Registration, RateLimitConfig {
            max_attempts: 3,
            window_duration: Duration::hours(1),
            block_duration: Some(Duration::hours(24)),
            burst_limit: Some(5),
            exponential_backoff: true,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 100,
            window_duration: Duration::minutes(1),
            block_duration: None,
            burst_limit: Some(150),
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        configs.insert(RateLimitType::AdminAction, RateLimitConfig {
            max_attempts: 20,
            window_duration: Duration::minutes(1),
            block_duration: Some(Duration::minutes(5)),
            burst_limit: Some(30),
            exponential_backoff: true,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        configs.insert(RateLimitType::DataExport, RateLimitConfig {
            max_attempts: 5,
            window_duration: Duration::hours(24),
            block_duration: Some(Duration::hours(2)),
            burst_limit: None,
            exponential_backoff: true,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        Self {
            users: Vec::new(),
            configs,
            ip_entries: HashMap::new(),
            user_entries: HashMap::new(),
            global_entries: HashMap::new(),
            violations: Vec::new(),
            metrics: RateLimitMetrics {
                total_requests: 0,
                blocked_requests: 0,
                violation_count: 0,
                avg_request_rate: 0.0,
                peak_request_rate: 0.0,
                top_violators: Vec::new(),
                block_effectiveness: 0.0,
            },
            adaptive_limits: HashMap::new(),
            circuit_breakers: HashMap::new(),
            distributed_state: None,
            geo_blocking: HashMap::new(),
            suspicious_patterns: HashMap::new(),
        }
    }

    pub async fn create_test_user(&mut self) -> Uuid {
        let user = User {
            id: Uuid::now_v7(),
            email: format!("user{}@example.com", Uuid::now_v7()),
            username: format!("user_{}", Uuid::now_v7()),
            status: UserStatus::Active,
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let user_id = user.id;
        self.users.push(user);
        user_id
    }

    pub async fn check_rate_limit_by_ip(
        &mut self,
        ip: IpAddr,
        limit_type: RateLimitType,
        source_info: Option<SourceInfo>,
    ) -> Result<(), AppError> {
        self.metrics.total_requests += 1;

        let config = self.configs.get(&limit_type)
            .ok_or_else(|| AppError::InternalServerError {
                message: "Rate limit config not found".to_string(),
            })?;

        // Check whitelist
        if config.whitelist_ips.contains(&ip) {
            return Ok(());
        }

        // Check geo-blocking
        if let Some(source) = &source_info {
            if let Some(location) = &source.geographic_location {
                if self.geo_blocking.get(location) == Some(&true) {
                    self.metrics.blocked_requests += 1;
                    return Err(AppError::TooManyRequests {
                        message: format!("Access blocked from location: {}", location),
                        retry_after: None,
                    });
                }
            }
        }

        // Check circuit breaker
        let circuit_key = format!("{}:{}", ip, format!("{:?}", limit_type));
        if let Some(circuit) = self.circuit_breakers.get(&circuit_key) {
            if circuit.state == CircuitState::Open {
                if let Some(last_failure) = circuit.last_failure {
                    if Utc::now() < last_failure + circuit.timeout_duration {
                        self.metrics.blocked_requests += 1;
                        return Err(AppError::TooManyRequests {
                            message: "Circuit breaker is open".to_string(),
                            retry_after: Some((last_failure + circuit.timeout_duration - Utc::now()).num_seconds() as u64),
                        });
                    }
                }
            }
        }

        let now = Utc::now();
        let key = (ip, limit_type);

        let entry = self.ip_entries.entry(key).or_insert_with(|| {
            RateLimitEntry {
                count: 0,
                window_start: now,
                last_request: now,
                blocked_until: None,
                backoff_multiplier: 1.0,
                burst_count: 0,
                burst_window_start: now,
            }
        });

        self.check_and_update_entry(entry, config, now, ip.to_string(), limit_type, source_info).await
    }

    pub async fn check_rate_limit_by_user(
        &mut self,
        user_id: Uuid,
        limit_type: RateLimitType,
        source_info: Option<SourceInfo>,
    ) -> Result<(), AppError> {
        self.metrics.total_requests += 1;

        let config = self.configs.get(&limit_type)
            .ok_or_else(|| AppError::InternalServerError {
                message: "Rate limit config not found".to_string(),
            })?;

        // Check priority users
        if config.priority_users.contains(&user_id) {
            return Ok(());
        }

        let now = Utc::now();
        let key = (user_id, limit_type);

        let entry = self.user_entries.entry(key).or_insert_with(|| {
            RateLimitEntry {
                count: 0,
                window_start: now,
                last_request: now,
                blocked_until: None,
                backoff_multiplier: 1.0,
                burst_count: 0,
                burst_window_start: now,
            }
        });

        self.check_and_update_entry(entry, config, now, user_id.to_string(), limit_type, source_info).await
    }

    async fn check_and_update_entry(
        &mut self,
        entry: &mut RateLimitEntry,
        config: &RateLimitConfig,
        now: DateTime<Utc>,
        identifier: String,
        limit_type: RateLimitType,
        source_info: Option<SourceInfo>,
    ) -> Result<(), AppError> {
        // Check if currently blocked
        if let Some(blocked_until) = entry.blocked_until {
            if now < blocked_until {
                self.metrics.blocked_requests += 1;
                let remaining = (blocked_until - now).num_seconds() as u64;
                return Err(AppError::TooManyRequests {
                    message: "Rate limited".to_string(),
                    retry_after: Some(remaining),
                });
            }
            // Block expired, reset entry
            entry.blocked_until = None;
            entry.count = 0;
            entry.window_start = now;
            entry.backoff_multiplier = 1.0;
        }

        // Check if window has expired
        if now > entry.window_start + config.window_duration {
            entry.count = 0;
            entry.window_start = now;
        }

        // Check burst limit
        if let Some(burst_limit) = config.burst_limit {
            if now > entry.burst_window_start + Duration::seconds(10) {
                entry.burst_count = 0;
                entry.burst_window_start = now;
            }
            entry.burst_count += 1;
            if entry.burst_count > burst_limit {
                self.record_violation(identifier.clone(), limit_type, entry.count, config.max_attempts, source_info.clone()).await;
                self.apply_burst_block(entry, config, now).await;
                self.metrics.blocked_requests += 1;
                return Err(AppError::TooManyRequests {
                    message: "Burst limit exceeded".to_string(),
                    retry_after: Some(60),
                });
            }
        }

        // Get effective limit (adaptive)
        let effective_limit = self.get_adaptive_limit(limit_type, config.max_attempts).await;

        // Increment count
        entry.count += 1;
        entry.last_request = now;

        // Check if limit exceeded
        if entry.count > effective_limit {
            self.record_violation(identifier.clone(), limit_type, entry.count, effective_limit, source_info.clone()).await;
            self.apply_block(entry, config, now).await;
            self.update_circuit_breaker(identifier, limit_type).await;
            self.detect_suspicious_patterns(identifier.clone(), now).await;
            self.metrics.blocked_requests += 1;

            let retry_after = if config.exponential_backoff {
                (config.block_duration.unwrap_or(Duration::minutes(1)).num_seconds() as f64 * entry.backoff_multiplier) as u64
            } else {
                config.block_duration.map(|d| d.num_seconds() as u64).unwrap_or(60)
            };

            return Err(AppError::TooManyRequests {
                message: format!("Rate limit exceeded. Maximum {} attempts allowed", effective_limit),
                retry_after: Some(retry_after),
            });
        }

        // Update peak rate tracking
        let current_rate = entry.count as f64 / config.window_duration.num_minutes() as f64;
        if current_rate > self.metrics.peak_request_rate {
            self.metrics.peak_request_rate = current_rate;
        }

        Ok(())
    }

    async fn apply_block(&mut self, entry: &mut RateLimitEntry, config: &RateLimitConfig, now: DateTime<Utc>) {
        if let Some(block_duration) = config.block_duration {
            let actual_duration = if config.exponential_backoff {
                Duration::milliseconds((block_duration.num_milliseconds() as f64 * entry.backoff_multiplier) as i64)
            } else {
                block_duration
            };

            entry.blocked_until = Some(now + actual_duration);

            if config.exponential_backoff {
                entry.backoff_multiplier = (entry.backoff_multiplier * 2.0).min(16.0); // Cap at 16x
            }
        }
    }

    async fn apply_burst_block(&mut self, entry: &mut RateLimitEntry, config: &RateLimitConfig, now: DateTime<Utc>) {
        // Temporary burst block
        entry.blocked_until = Some(now + Duration::seconds(60));
    }

    async fn get_adaptive_limit(&mut self, limit_type: RateLimitType, base_limit: u32) -> u32 {
        if let Some(adaptive) = self.adaptive_limits.get_mut(&limit_type) {
            let load_factor = self.calculate_current_load().await;

            if load_factor > adaptive.load_threshold {
                // Reduce limit under high load
                adaptive.current_limit = ((base_limit as f64) * adaptive.adjustment_factor).max(1.0) as u32;
            } else {
                // Restore to base limit under normal load
                adaptive.current_limit = base_limit;
            }

            adaptive.last_adjustment = Utc::now();
            adaptive.current_limit
        } else {
            // Initialize adaptive limit
            self.adaptive_limits.insert(limit_type, AdaptiveRateLimit {
                base_limit,
                current_limit: base_limit,
                adjustment_factor: 0.7,
                load_threshold: 0.8,
                last_adjustment: Utc::now(),
            });
            base_limit
        }
    }

    async fn calculate_current_load(&self) -> f64 {
        // Simulate system load calculation
        let active_entries = self.ip_entries.len() + self.user_entries.len();
        let load = active_entries as f64 / 1000.0; // Simulate capacity
        load.min(1.0) // Cap at 100%
    }

    async fn record_violation(
        &mut self,
        identifier: String,
        limit_type: RateLimitType,
        attempts_made: u32,
        limit_exceeded: u32,
        source_info: Option<SourceInfo>,
    ) {
        let violation = RateLimitViolation {
            id: Uuid::now_v7(),
            identifier: identifier.clone(),
            limit_type,
            violation_time: Utc::now(),
            attempts_made,
            limit_exceeded,
            source_info: source_info.unwrap_or(SourceInfo {
                ip_address: "0.0.0.0".parse().unwrap(),
                user_agent: None,
                geographic_location: None,
                isp: None,
                device_fingerprint: None,
            }),
        };

        self.violations.push(violation);
        self.metrics.violation_count += 1;

        // Update top violators
        self.update_top_violators(identifier).await;
    }

    async fn update_top_violators(&mut self, identifier: String) {
        // Find existing violator or add new one
        if let Some(pos) = self.metrics.top_violators.iter().position(|(id, _)| id == &identifier) {
            self.metrics.top_violators[pos].1 += 1;
        } else {
            self.metrics.top_violators.push((identifier, 1));
        }

        // Sort and keep top 10
        self.metrics.top_violators.sort_by(|a, b| b.1.cmp(&a.1));
        self.metrics.top_violators.truncate(10);
    }

    async fn update_circuit_breaker(&mut self, identifier: String, limit_type: RateLimitType) {
        let circuit_key = format!("{}:{:?}", identifier, limit_type);

        let circuit = self.circuit_breakers.entry(circuit_key).or_insert_with(|| CircuitBreakerState {
            failure_count: 0,
            failure_threshold: 5,
            timeout_duration: Duration::minutes(2),
            last_failure: None,
            state: CircuitState::Closed,
        });

        circuit.failure_count += 1;
        circuit.last_failure = Some(Utc::now());

        if circuit.failure_count >= circuit.failure_threshold {
            circuit.state = CircuitState::Open;
        }
    }

    async fn detect_suspicious_patterns(&mut self, identifier: String, timestamp: DateTime<Utc>) {
        let pattern_key = format!("rapid_requests:{}", identifier);
        let requests = self.suspicious_patterns.entry(pattern_key).or_insert_with(Vec::new);

        requests.push(timestamp);

        // Keep only last minute of requests
        let minute_ago = timestamp - Duration::minutes(1);
        requests.retain(|&ts| ts > minute_ago);

        // Flag as suspicious if more than 20 requests in a minute
        if requests.len() > 20 {
            // In a real system, this would trigger additional security measures
            self.apply_security_measures(identifier, "rapid_requests").await;
        }
    }

    async fn apply_security_measures(&mut self, identifier: String, pattern: &str) {
        // Simulate security measures like temporary IP blocking, CAPTCHA requirements, etc.
        match pattern {
            "rapid_requests" => {
                // Could trigger CAPTCHA or temporary IP block
            },
            "geographic_anomaly" => {
                // Could trigger additional verification
            },
            _ => {}
        }
    }

    pub async fn enable_geo_blocking(&mut self, country_code: String, blocked: bool) {
        self.geo_blocking.insert(country_code, blocked);
    }

    pub async fn add_whitelist_ip(&mut self, limit_type: RateLimitType, ip: IpAddr) -> Result<(), AppError> {
        if let Some(config) = self.configs.get_mut(&limit_type) {
            config.whitelist_ips.insert(ip);
            Ok(())
        } else {
            Err(AppError::BadRequest {
                message: "Invalid rate limit type".to_string(),
            })
        }
    }

    pub async fn add_priority_user(&mut self, limit_type: RateLimitType, user_id: Uuid) -> Result<(), AppError> {
        if let Some(config) = self.configs.get_mut(&limit_type) {
            config.priority_users.insert(user_id);
            Ok(())
        } else {
            Err(AppError::BadRequest {
                message: "Invalid rate limit type".to_string(),
            })
        }
    }

    pub async fn get_rate_limit_metrics(&self) -> RateLimitMetrics {
        let mut metrics = self.metrics.clone();

        // Calculate effectiveness
        if metrics.total_requests > 0 {
            metrics.block_effectiveness = (metrics.blocked_requests as f64 / metrics.total_requests as f64) * 100.0;
        }

        // Calculate average rate
        if metrics.total_requests > 0 {
            let duration_hours = 1.0; // Simulate 1 hour period
            metrics.avg_request_rate = metrics.total_requests as f64 / duration_hours;
        }

        metrics
    }

    pub async fn get_violations(&self, limit: Option<usize>) -> Vec<RateLimitViolation> {
        match limit {
            Some(n) => self.violations.iter().rev().take(n).cloned().collect(),
            None => self.violations.clone(),
        }
    }

    pub async fn cleanup_expired_entries(&mut self) -> usize {
        let now = Utc::now();
        let mut removed = 0;

        // Clean IP entries
        self.ip_entries.retain(|_, entry| {
            let should_keep = if let Some(blocked_until) = entry.blocked_until {
                now < blocked_until + Duration::hours(1)
            } else {
                now < entry.last_request + Duration::hours(24)
            };
            if !should_keep {
                removed += 1;
            }
            should_keep
        });

        // Clean user entries
        self.user_entries.retain(|_, entry| {
            let should_keep = now < entry.last_request + Duration::hours(24);
            if !should_keep {
                removed += 1;
            }
            should_keep
        });

        // Clean old violations (keep last 1000)
        if self.violations.len() > 1000 {
            let excess = self.violations.len() - 1000;
            self.violations.drain(0..excess);
            removed += excess;
        }

        // Clean old suspicious patterns
        for patterns in self.suspicious_patterns.values_mut() {
            let hour_ago = now - Duration::hours(1);
            let before_len = patterns.len();
            patterns.retain(|&ts| ts > hour_ago);
            removed += before_len - patterns.len();
        }

        removed
    }

    pub async fn reset_circuit_breaker(&mut self, circuit_key: String) -> Result<(), AppError> {
        if let Some(circuit) = self.circuit_breakers.get_mut(&circuit_key) {
            circuit.failure_count = 0;
            circuit.state = CircuitState::Closed;
            circuit.last_failure = None;
            Ok(())
        } else {
            Err(AppError::NotFound {
                resource: "circuit_breaker".to_string(),
                id: Some(circuit_key),
            })
        }
    }

    pub async fn simulate_distributed_rate_limiting(&mut self, node_id: String) -> Result<(), AppError> {
        self.distributed_state = Some(DistributedRateLimit {
            node_id,
            global_count: Arc::new(RwLock::new(HashMap::new())),
            sync_interval: Duration::seconds(30),
            last_sync: Utc::now(),
        });
        Ok(())
    }

    pub async fn test_rate_limit_scalability(&mut self, concurrent_requests: usize) -> Result<Duration, AppError> {
        let start_time = std::time::Instant::now();
        let mut successful = 0;
        let mut blocked = 0;

        for i in 0..concurrent_requests {
            let ip: IpAddr = format!("192.168.{}.{}", i / 256, i % 256).parse().unwrap();

            let source_info = Some(SourceInfo {
                ip_address: ip,
                user_agent: Some("LoadTest/1.0".to_string()),
                geographic_location: Some("US".to_string()),
                isp: Some("TestISP".to_string()),
                device_fingerprint: Some(format!("device_{}", i)),
            });

            match self.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, source_info).await {
                Ok(_) => successful += 1,
                Err(_) => blocked += 1,
            }

            // Early exit if taking too long
            if start_time.elapsed() > std::time::Duration::from_secs(10) {
                break;
            }
        }

        let duration = Duration::milliseconds(start_time.elapsed().as_millis() as i64);
        println!("Processed {} requests: {} successful, {} blocked in {}ms",
                 successful + blocked, successful, blocked, duration.num_milliseconds());

        Ok(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_rate_limiting_basic() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Test basic rate limiting with source info
        let source_info = Some(SourceInfo {
            ip_address: ip,
            user_agent: Some("TestBrowser/1.0".to_string()),
            geographic_location: Some("US".to_string()),
            isp: Some("TestISP".to_string()),
            device_fingerprint: Some("test_device".to_string()),
        });

        // First 5 login attempts should succeed
        for i in 1..=5 {
            let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info.clone()).await;
            assert!(result.is_ok(), "Attempt {} should succeed", i);
        }

        // 6th attempt should fail
        let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info.clone()).await;
        assert!(result.is_err());

        // Verify metrics were updated
        let metrics = simulator.get_rate_limit_metrics().await;
        assert_eq!(metrics.total_requests, 6);
        assert_eq!(metrics.blocked_requests, 1);
        assert_eq!(metrics.violation_count, 1);
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.2".parse().unwrap();

        // Configure short limits for testing
        simulator.configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(1),
            block_duration: Some(Duration::seconds(1)),
            burst_limit: None,
            exponential_backoff: true,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        // Exceed limit
        simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();
        simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();

        let first_block = simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await;
        assert!(first_block.is_err());

        // Wait for block to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Exceed again - should have longer block due to exponential backoff
        simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();
        simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();

        let second_block = simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await;
        assert!(second_block.is_err());

        // Verify backoff multiplier increased
        if let Some(entry) = simulator.ip_entries.get(&(ip, RateLimitType::ApiRequest)) {
            assert!(entry.backoff_multiplier > 1.0);
        }
    }

    #[tokio::test]
    async fn test_burst_limiting() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.3".parse().unwrap();

        // Configure with burst limit
        simulator.configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 100,
            window_duration: Duration::minutes(1),
            block_duration: None,
            burst_limit: Some(5), // Only 5 requests in burst window
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        // Make burst requests
        for i in 1..=5 {
            let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await;
            assert!(result.is_ok(), "Burst attempt {} should succeed", i);
        }

        // Next request should fail due to burst limit
        let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await;
        assert!(result.is_err());

        if let Err(AppError::TooManyRequests { message, .. }) = result {
            assert!(message.contains("Burst limit exceeded"));
        }
    }

    #[tokio::test]
    async fn test_whitelist_functionality() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let whitelisted_ip: IpAddr = "10.0.0.1".parse().unwrap();
        let normal_ip: IpAddr = "192.168.1.4".parse().unwrap();

        // Add IP to whitelist
        simulator.add_whitelist_ip(RateLimitType::Login, whitelisted_ip).await.unwrap();

        // Whitelisted IP should never be blocked
        for _ in 0..10 {
            let result = simulator.check_rate_limit_by_ip(whitelisted_ip, RateLimitType::Login, None).await;
            assert!(result.is_ok(), "Whitelisted IP should never be blocked");
        }

        // Normal IP should be blocked after limit
        for _ in 0..5 {
            simulator.check_rate_limit_by_ip(normal_ip, RateLimitType::Login, None).await.ok();
        }
        let result = simulator.check_rate_limit_by_ip(normal_ip, RateLimitType::Login, None).await;
        assert!(result.is_err(), "Normal IP should be blocked after limit");
    }

    #[tokio::test]
    async fn test_priority_users() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let priority_user = simulator.create_test_user().await;
        let normal_user = simulator.create_test_user().await;

        // Add user to priority list
        simulator.add_priority_user(RateLimitType::ApiRequest, priority_user).await.unwrap();

        // Configure low limits for testing
        simulator.configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(1),
            block_duration: None,
            burst_limit: None,
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::from([priority_user]),
        });

        // Priority user should never be blocked
        for _ in 0..10 {
            let result = simulator.check_rate_limit_by_user(priority_user, RateLimitType::ApiRequest, None).await;
            assert!(result.is_ok(), "Priority user should never be blocked");
        }

        // Normal user should be blocked after limit
        simulator.check_rate_limit_by_user(normal_user, RateLimitType::ApiRequest, None).await.ok();
        simulator.check_rate_limit_by_user(normal_user, RateLimitType::ApiRequest, None).await.ok();
        let result = simulator.check_rate_limit_by_user(normal_user, RateLimitType::ApiRequest, None).await;
        assert!(result.is_err(), "Normal user should be blocked after limit");
    }

    #[tokio::test]
    async fn test_geo_blocking() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.5".parse().unwrap();

        // Enable geo-blocking for specific country
        simulator.enable_geo_blocking("XX".to_string(), true).await;

        let source_info = Some(SourceInfo {
            ip_address: ip,
            user_agent: Some("Browser/1.0".to_string()),
            geographic_location: Some("XX".to_string()), // Blocked country
            isp: Some("ISP".to_string()),
            device_fingerprint: Some("device".to_string()),
        });

        // Request from blocked country should fail
        let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info).await;
        assert!(result.is_err());

        if let Err(AppError::TooManyRequests { message, .. }) = result {
            assert!(message.contains("Access blocked from location"));
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.6".parse().unwrap();

        // Configure to trigger circuit breaker quickly
        simulator.configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 1,
            window_duration: Duration::minutes(1),
            block_duration: Some(Duration::seconds(1)),
            burst_limit: None,
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        // Trigger multiple failures to open circuit breaker
        for _ in 0..6 {
            simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();
            simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.err();
            tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await; // Wait for block to expire
        }

        // Circuit breaker should now be open
        let circuit_key = format!("{}:{:?}", ip, RateLimitType::ApiRequest);
        if let Some(circuit) = simulator.circuit_breakers.get(&circuit_key) {
            assert_eq!(circuit.state, CircuitState::Open);
        }

        // Requests should be blocked by circuit breaker
        let result = simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await;
        assert!(result.is_err());

        if let Err(AppError::TooManyRequests { message, .. }) = result {
            assert!(message.contains("Circuit breaker is open"));
        }
    }

    #[tokio::test]
    async fn test_adaptive_rate_limiting() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.7".parse().unwrap();

        // Simulate high load to trigger adaptive limiting
        for _ in 0..1500 {
            let test_ip: IpAddr = format!("10.0.{}.{}", rand::random::<u8>(), rand::random::<u8>()).parse().unwrap();
            simulator.check_rate_limit_by_ip(test_ip, RateLimitType::ApiRequest, None).await.ok();
        }

        // Check that adaptive limit was reduced
        let adaptive_limit = simulator.get_adaptive_limit(RateLimitType::ApiRequest, 100).await;
        assert!(adaptive_limit < 100, "Adaptive limit should be reduced under high load");
    }

    #[tokio::test]
    async fn test_suspicious_pattern_detection() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.8".parse().unwrap();

        // Configure high limit to focus on pattern detection
        simulator.configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 1000,
            window_duration: Duration::minutes(1),
            block_duration: None,
            burst_limit: None,
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        // Make rapid requests to trigger pattern detection
        for _ in 0..25 {
            simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();
        }

        // Check that suspicious pattern was detected
        let pattern_key = format!("rapid_requests:{}", ip);
        assert!(simulator.suspicious_patterns.contains_key(&pattern_key));
    }

    #[tokio::test]
    async fn test_violation_tracking() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.9".parse().unwrap();

        // Configure low limits to trigger violations
        simulator.configs.insert(RateLimitType::Login, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(1),
            block_duration: None,
            burst_limit: None,
            exponential_backoff: false,
            whitelist_ips: HashSet::new(),
            priority_users: HashSet::new(),
        });

        let source_info = Some(SourceInfo {
            ip_address: ip,
            user_agent: Some("TestBrowser/1.0".to_string()),
            geographic_location: Some("US".to_string()),
            isp: Some("TestISP".to_string()),
            device_fingerprint: Some("test_device".to_string()),
        });

        // Trigger violations
        simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info.clone()).await.ok();
        simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info.clone()).await.ok();
        simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, source_info.clone()).await.err();

        // Check violations were recorded
        let violations = simulator.get_violations(None).await;
        assert!(!violations.is_empty());

        let violation = &violations[0];
        assert_eq!(violation.limit_type, RateLimitType::Login);
        assert!(violation.source_info.user_agent.is_some());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.10".parse().unwrap();

        // Make some requests
        for _ in 0..10 {
            simulator.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest, None).await.ok();
        }

        // Get metrics
        let metrics = simulator.get_rate_limit_metrics().await;

        assert!(metrics.total_requests > 0);
        assert!(metrics.avg_request_rate >= 0.0);
        assert!(metrics.block_effectiveness >= 0.0);
    }

    #[tokio::test]
    async fn test_resource_cleanup() {
        let mut simulator = RateLimitingBehaviorSimulator::new();
        let ip: IpAddr = "192.168.1.11".parse().unwrap();

        // Create entries
        simulator.check_rate_limit_by_ip(ip, RateLimitType::Login, None).await.ok();

        // Age the entries
        if let Some(entry) = simulator.ip_entries.get_mut(&(ip, RateLimitType::Login)) {
            entry.last_request = Utc::now() - Duration::hours(25);
        }

        // Add old violations
        for _ in 0..1100 {
            simulator.violations.push(RateLimitViolation {
                id: Uuid::now_v7(),
                identifier: "test".to_string(),
                limit_type: RateLimitType::Login,
                violation_time: Utc::now() - Duration::hours(1),
                attempts_made: 5,
                limit_exceeded: 3,
                source_info: SourceInfo {
                    ip_address: ip,
                    user_agent: None,
                    geographic_location: None,
                    isp: None,
                    device_fingerprint: None,
                },
            });
        }

        let initial_violations = simulator.violations.len();
        let cleaned = simulator.cleanup_expired_entries().await;

        assert!(cleaned > 0);
        assert!(simulator.violations.len() <= 1000);
        assert!(simulator.violations.len() < initial_violations);
    }

    #[tokio::test]
    async fn test_scalability() {
        let mut simulator = RateLimitingBehaviorSimulator::new();

        // Test scalability with concurrent requests
        let duration = simulator.test_rate_limit_scalability(500).await.unwrap();

        assert!(duration.num_milliseconds() > 0);

        let metrics = simulator.get_rate_limit_metrics().await;
        assert!(metrics.total_requests > 0);
    }
}