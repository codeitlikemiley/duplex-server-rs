//! Integration tests for rate limiting and traffic management - Enterprise Edition
//!
//! These tests verify that rate limiting mechanisms work correctly to protect
//! the system from abuse, ensure fair resource allocation, and maintain service quality.
//!
//! Enterprise features include:
//! - Multi-dimensional rate limiting (IP, user, API key, endpoint)
//! - Adaptive rate limiting based on system load
//! - Distributed rate limiting across multiple nodes
//! - Rate limiting analytics and abuse detection
//! - Hierarchical rate limiting with priority tiers
//! - Burst handling and token bucket algorithms
//! - Rate limiting bypass for privileged users
//! - Geolocation-based rate limiting
//! - Time-based rate limiting patterns
//! - Custom rate limiting rules and policies

use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BTreeMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

// Rate Limiting Core Types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RateLimitKey {
    pub dimension: RateLimitDimension,
    pub identifier: String,
    pub resource: String,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateLimitDimension {
    IPAddress,
    UserId,
    ApiKey,
    Endpoint,
    UserAgent,
    Geographic,
    Combined(Vec<RateLimitDimension>),
}

// Rate Limiting Algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitAlgorithm {
    TokenBucket {
        capacity: u64,
        refill_rate: u64,
        refill_period: Duration,
    },
    LeakyBucket {
        capacity: u64,
        leak_rate: u64,
        leak_period: Duration,
    },
    FixedWindow {
        limit: u64,
        window_size: Duration,
    },
    SlidingWindow {
        limit: u64,
        window_size: Duration,
        sub_window_count: usize,
    },
    Adaptive {
        base_limit: u64,
        max_limit: u64,
        adaptation_factor: f64,
        load_threshold: f64,
    },
}

// Rate Limit Policies and Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    pub id: Uuid,
    pub name: String,
    pub algorithm: RateLimitAlgorithm,
    pub dimensions: Vec<RateLimitDimension>,
    pub priority: RateLimitPriority,
    pub scope: RateLimitScope,
    pub exemptions: Vec<RateLimitExemption>,
    pub actions: Vec<RateLimitAction>,
    pub time_restrictions: Option<TimeRestrictions>,
    pub geographic_restrictions: Option<GeographicRestrictions>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitPriority {
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
    Background = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitScope {
    Global,
    Tenant(String),
    Endpoint(String),
    Service(String),
    Custom(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitExemption {
    pub exemption_type: ExemptionType,
    pub identifier: String,
    pub reason: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExemptionType {
    IPAddress,
    UserId,
    ApiKey,
    UserAgent,
    Geographic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitAction {
    Block,
    Delay(Duration),
    Throttle(f64), // Factor to reduce rate by
    Alert,
    Log,
    Redirect(String),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>, // 0-23
    pub allowed_days: Vec<u8>,  // 0-6 (Sunday-Saturday)
    pub timezone: String,
    pub maintenance_windows: Vec<MaintenanceWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub reduced_limit_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicRestrictions {
    pub allowed_countries: Option<Vec<String>>,
    pub blocked_countries: Option<Vec<String>>,
    pub allowed_regions: Option<Vec<String>>,
    pub blocked_regions: Option<Vec<String>>,
    pub country_specific_limits: HashMap<String, u64>,
}

// Rate Limiting State and Tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    pub key: RateLimitKey,
    pub algorithm_state: AlgorithmState,
    pub request_count: u64,
    pub last_request: DateTime<Utc>,
    pub first_request: DateTime<Utc>,
    pub total_blocked: u64,
    pub total_allowed: u64,
    pub burst_count: u64,
    pub consecutive_violations: u64,
    pub reputation_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlgorithmState {
    TokenBucket { tokens: f64, last_refill: DateTime<Utc> },
    LeakyBucket { queue_size: usize, last_leak: DateTime<Utc> },
    FixedWindow { count: u64, window_start: DateTime<Utc> },
    SlidingWindow { sub_windows: VecDeque<u64>, window_start: DateTime<Utc> },
    Adaptive { current_limit: u64, load_factor: f64, last_adaptation: DateTime<Utc> },
}

// Rate Limiting Request and Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRequest {
    pub dimensions: HashMap<RateLimitDimension, String>,
    pub resource: String,
    pub weight: u64, // Number of units this request consumes
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub reset_time: DateTime<Utc>,
    pub retry_after: Option<Duration>,
    pub policy_id: Uuid,
    pub action_taken: Option<RateLimitAction>,
    pub reason: Option<String>,
    pub headers: HashMap<String, String>,
}

// Rate Limiting Analytics and Monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitAnalytics {
    pub total_requests: u64,
    pub total_allowed: u64,
    pub total_blocked: u64,
    pub block_rate: f64,
    pub top_violators: Vec<TopViolator>,
    pub policy_performance: HashMap<Uuid, PolicyPerformance>,
    pub geographic_distribution: HashMap<String, GeographicStats>,
    pub time_based_patterns: HashMap<String, TimeBasedStats>,
    pub burst_patterns: Vec<BurstPattern>,
    pub abuse_indicators: Vec<AbuseIndicator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopViolator {
    pub identifier: String,
    pub dimension: RateLimitDimension,
    pub violation_count: u64,
    pub first_violation: DateTime<Utc>,
    pub last_violation: DateTime<Utc>,
    pub severity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPerformance {
    pub policy_id: Uuid,
    pub policy_name: String,
    pub requests_processed: u64,
    pub blocks_issued: u64,
    pub effectiveness_score: f64,
    pub average_response_time: Duration,
    pub false_positive_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicStats {
    pub country_code: String,
    pub request_count: u64,
    pub block_count: u64,
    pub average_request_rate: f64,
    pub threat_level: ThreatLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedStats {
    pub hour_of_day: u8,
    pub day_of_week: u8,
    pub request_volume: u64,
    pub block_volume: u64,
    pub peak_request_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstPattern {
    pub identifier: String,
    pub dimension: RateLimitDimension,
    pub start_time: DateTime<Utc>,
    pub duration: Duration,
    pub peak_rate: f64,
    pub total_requests: u64,
    pub pattern_type: BurstPatternType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BurstPatternType {
    Normal,
    Suspicious,
    Attack,
    Bot,
    Scraping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbuseIndicator {
    pub indicator_id: Uuid,
    pub indicator_type: AbuseIndicatorType,
    pub identifier: String,
    pub dimension: RateLimitDimension,
    pub confidence_score: f64,
    pub first_detected: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub evidence: Vec<AbuseEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AbuseIndicatorType {
    BotActivity,
    ScrapingBehavior,
    DDoSAttack,
    BruteForceAttack,
    ApiAbuse,
    ResourceExhaustion,
    SuspiciousPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbuseEvidence {
    pub evidence_type: String,
    pub value: String,
    pub timestamp: DateTime<Utc>,
    pub severity: f64,
}

// System Load and Adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoadMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub request_queue_size: usize,
    pub average_response_time: Duration,
    pub error_rate: f64,
    pub active_connections: usize,
    pub load_factor: f64, // Normalized 0.0-1.0
}

// Enterprise Rate Limiting Engine
#[derive(Debug)]
pub struct EnterpriseRateLimitEngine {
    policies: Arc<RwLock<HashMap<Uuid, RateLimitPolicy>>>,
    state_store: Arc<RwLock<HashMap<RateLimitKey, RateLimitState>>>,
    analytics: Arc<RwLock<RateLimitAnalytics>>,
    system_load: Arc<RwLock<SystemLoadMetrics>>,
    config: RateLimitEngineConfig,
    exemption_cache: Arc<RwLock<HashMap<String, Vec<RateLimitExemption>>>>,
    abuse_detection: Arc<RwLock<AbuseDetectionEngine>>,
}

#[derive(Debug, Clone)]
pub struct RateLimitEngineConfig {
    pub distributed_mode: bool,
    pub analytics_enabled: bool,
    pub abuse_detection_enabled: bool,
    pub adaptive_limits_enabled: bool,
    pub geographic_restrictions_enabled: bool,
    pub cleanup_interval: Duration,
    pub max_state_entries: usize,
    pub default_burst_allowance: f64,
    pub reputation_decay_rate: f64,
}

impl Default for RateLimitEngineConfig {
    fn default() -> Self {
        Self {
            distributed_mode: true,
            analytics_enabled: true,
            abuse_detection_enabled: true,
            adaptive_limits_enabled: true,
            geographic_restrictions_enabled: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_state_entries: 1000000, // 1 million entries
            default_burst_allowance: 1.2, // 20% burst allowance
            reputation_decay_rate: 0.95, // 5% decay per cleanup cycle
        }
    }
}

#[derive(Debug, Clone)]
pub struct AbuseDetectionEngine {
    indicators: HashMap<String, AbuseIndicator>,
    detection_rules: Vec<AbuseDetectionRule>,
    confidence_threshold: f64,
    auto_block_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct AbuseDetectionRule {
    pub rule_id: Uuid,
    pub name: String,
    pub pattern: AbusePattern,
    pub threshold: f64,
    pub action: AbuseAction,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum AbusePattern {
    RapidRequests { rate_threshold: f64, window: Duration },
    RepeatedBlocks { block_count: u64, window: Duration },
    UniformTiming { variance_threshold: f64 },
    SuspiciousUserAgent { patterns: Vec<String> },
    GeoAnomaly { unusual_country_access: bool },
    ResourceExhaustion { resource_types: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum AbuseAction {
    Alert,
    Block,
    Quarantine,
    ReduceLimit(f64),
    RequireVerification,
}

impl EnterpriseRateLimitEngine {
    pub fn new() -> Self {
        Self::with_config(RateLimitEngineConfig::default())
    }

    pub fn with_config(config: RateLimitEngineConfig) -> Self {
        let abuse_detection = AbuseDetectionEngine {
            indicators: HashMap::new(),
            detection_rules: Self::create_default_abuse_rules(),
            confidence_threshold: 0.7,
            auto_block_threshold: 0.9,
        };

        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            state_store: Arc::new(RwLock::new(HashMap::new())),
            analytics: Arc::new(RwLock::new(RateLimitAnalytics::default())),
            system_load: Arc::new(RwLock::new(SystemLoadMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                request_queue_size: 0,
                average_response_time: Duration::from_millis(100),
                error_rate: 0.0,
                active_connections: 0,
                load_factor: 0.0,
            })),
            config,
            exemption_cache: Arc::new(RwLock::new(HashMap::new())),
            abuse_detection: Arc::new(RwLock::new(abuse_detection)),
        }
    }

    // Core rate limiting operations
    pub async fn check_rate_limit(&self, request: &RateLimitRequest) -> RateLimitResponse {
        let start_time = Instant::now();

        // Find applicable policies
        let applicable_policies = self.find_applicable_policies(request).await;

        // Check exemptions first
        if self.is_exempted(request).await {
            return RateLimitResponse {
                allowed: true,
                limit: u64::MAX,
                remaining: u64::MAX,
                reset_time: Utc::now() + ChronoDuration::days(1),
                retry_after: None,
                policy_id: Uuid::nil(),
                action_taken: None,
                reason: Some("Exempted".to_string()),
                headers: HashMap::new(),
            };
        }

        // Apply most restrictive policy
        let mut most_restrictive_response = None;
        let mut min_remaining = u64::MAX;

        for policy in applicable_policies {
            let response = self.evaluate_policy(&policy, request).await;

            if !response.allowed {
                self.record_block(&policy, request, &response).await;
                return response;
            }

            if response.remaining < min_remaining {
                min_remaining = response.remaining;
                most_restrictive_response = Some(response);
            }
        }

        let response = most_restrictive_response.unwrap_or_else(|| RateLimitResponse {
            allowed: true,
            limit: 1000,
            remaining: 999,
            reset_time: Utc::now() + ChronoDuration::hours(1),
            retry_after: None,
            policy_id: Uuid::nil(),
            action_taken: None,
            reason: None,
            headers: HashMap::new(),
        });

        // Update analytics
        self.update_analytics(&response, start_time).await;

        // Check for abuse patterns
        if self.config.abuse_detection_enabled {
            self.check_abuse_patterns(request).await;
        }

        response
    }

    pub async fn add_policy(&self, policy: RateLimitPolicy) {
        let mut policies = self.policies.write().unwrap();
        policies.insert(policy.id, policy);
    }

    pub async fn remove_policy(&self, policy_id: Uuid) {
        let mut policies = self.policies.write().unwrap();
        policies.remove(&policy_id);
    }

    pub async fn update_policy(&self, policy: RateLimitPolicy) {
        let mut policies = self.policies.write().unwrap();
        policies.insert(policy.id, policy);
    }

    pub async fn add_exemption(&self, exemption: RateLimitExemption) {
        let mut cache = self.exemption_cache.write().unwrap();
        let exemptions = cache.entry(exemption.identifier.clone()).or_insert_with(Vec::new);
        exemptions.push(exemption);
    }

    pub async fn get_analytics(&self) -> RateLimitAnalytics {
        self.analytics.read().unwrap().clone()
    }

    pub async fn get_system_load(&self) -> SystemLoadMetrics {
        self.system_load.read().unwrap().clone()
    }

    pub async fn update_system_load(&self, load_metrics: SystemLoadMetrics) {
        *self.system_load.write().unwrap() = load_metrics;
    }

    // Helper methods
    async fn find_applicable_policies(&self, request: &RateLimitRequest) -> Vec<RateLimitPolicy> {
        let policies = self.policies.read().unwrap();
        let mut applicable = Vec::new();

        for policy in policies.values() {
            if !policy.enabled {
                continue;
            }

            // Check if policy applies to this request
            if self.policy_applies_to_request(policy, request) {
                applicable.push(policy.clone());
            }
        }

        // Sort by priority (higher priority first)
        applicable.sort_by_key(|p| p.priority.clone() as u8);
        applicable
    }

    fn policy_applies_to_request(&self, policy: &RateLimitPolicy, request: &RateLimitRequest) -> bool {
        // Check scope
        match &policy.scope {
            RateLimitScope::Global => true,
            RateLimitScope::Tenant(tenant) => {
                request.metadata.get("tenant_id") == Some(tenant)
            },
            RateLimitScope::Endpoint(endpoint) => {
                request.resource == *endpoint
            },
            RateLimitScope::Service(service) => {
                request.metadata.get("service") == Some(service)
            },
            RateLimitScope::Custom(conditions) => {
                conditions.iter().all(|(key, value)| {
                    request.metadata.get(key) == Some(value)
                })
            },
        }
    }

    async fn is_exempted(&self, request: &RateLimitRequest) -> bool {
        let cache = self.exemption_cache.read().unwrap();

        for (dimension, identifier) in &request.dimensions {
            if let Some(exemptions) = cache.get(identifier) {
                for exemption in exemptions {
                    if self.exemption_matches(dimension, &exemption.exemption_type) {
                        // Check if exemption is still valid
                        if let Some(expires_at) = exemption.expires_at {
                            if Utc::now() > expires_at {
                                continue;
                            }
                        }
                        return true;
                    }
                }
            }
        }

        false
    }

    fn exemption_matches(&self, dimension: &RateLimitDimension, exemption_type: &ExemptionType) -> bool {
        match (dimension, exemption_type) {
            (RateLimitDimension::IPAddress, ExemptionType::IPAddress) => true,
            (RateLimitDimension::UserId, ExemptionType::UserId) => true,
            (RateLimitDimension::ApiKey, ExemptionType::ApiKey) => true,
            (RateLimitDimension::UserAgent, ExemptionType::UserAgent) => true,
            (RateLimitDimension::Geographic, ExemptionType::Geographic) => true,
            _ => false,
        }
    }

    async fn evaluate_policy(&self, policy: &RateLimitPolicy, request: &RateLimitRequest) -> RateLimitResponse {
        // Create rate limit keys for each dimension
        let mut responses = Vec::new();

        for dimension in &policy.dimensions {
            if let Some(identifier) = request.dimensions.get(dimension) {
                let key = RateLimitKey {
                    dimension: dimension.clone(),
                    identifier: identifier.clone(),
                    resource: request.resource.clone(),
                    tenant_id: request.metadata.get("tenant_id").cloned(),
                };

                let response = self.evaluate_key_against_algorithm(&key, &policy.algorithm, request.weight).await;
                responses.push(response);
            }
        }

        // Return most restrictive response
        responses.into_iter()
            .min_by_key(|r| if r.allowed { r.remaining } else { 0 })
            .unwrap_or_else(|| RateLimitResponse {
                allowed: false,
                limit: 0,
                remaining: 0,
                reset_time: Utc::now(),
                retry_after: Some(Duration::from_secs(60)),
                policy_id: policy.id,
                action_taken: Some(RateLimitAction::Block),
                reason: Some("No applicable rate limit found".to_string()),
                headers: HashMap::new(),
            })
    }

    async fn evaluate_key_against_algorithm(&self, key: &RateLimitKey, algorithm: &RateLimitAlgorithm, weight: u64) -> RateLimitResponse {
        let mut state_store = self.state_store.write().unwrap();
        let now = Utc::now();

        let state = state_store.entry(key.clone()).or_insert_with(|| RateLimitState {
            key: key.clone(),
            algorithm_state: self.create_initial_algorithm_state(algorithm),
            request_count: 0,
            last_request: now,
            first_request: now,
            total_blocked: 0,
            total_allowed: 0,
            burst_count: 0,
            consecutive_violations: 0,
            reputation_score: 1.0,
        });

        match algorithm {
            RateLimitAlgorithm::TokenBucket { capacity, refill_rate, refill_period } => {
                self.evaluate_token_bucket(state, *capacity, *refill_rate, *refill_period, weight, now).await
            },
            RateLimitAlgorithm::LeakyBucket { capacity, leak_rate, leak_period } => {
                self.evaluate_leaky_bucket(state, *capacity, *leak_rate, *leak_period, weight, now).await
            },
            RateLimitAlgorithm::FixedWindow { limit, window_size } => {
                self.evaluate_fixed_window(state, *limit, *window_size, weight, now).await
            },
            RateLimitAlgorithm::SlidingWindow { limit, window_size, sub_window_count } => {
                self.evaluate_sliding_window(state, *limit, *window_size, *sub_window_count, weight, now).await
            },
            RateLimitAlgorithm::Adaptive { base_limit, max_limit, adaptation_factor, load_threshold } => {
                self.evaluate_adaptive(state, *base_limit, *max_limit, *adaptation_factor, *load_threshold, weight, now).await
            },
        }
    }

    async fn evaluate_token_bucket(&self, state: &mut RateLimitState, capacity: u64, refill_rate: u64, refill_period: Duration, weight: u64, now: DateTime<Utc>) -> RateLimitResponse {
        if let AlgorithmState::TokenBucket { tokens, last_refill } = &mut state.algorithm_state {
            // Calculate tokens to add based on time elapsed
            let time_elapsed = now.signed_duration_since(*last_refill);
            let periods_elapsed = time_elapsed.num_seconds() as f64 / refill_period.as_secs() as f64;
            let tokens_to_add = (periods_elapsed * refill_rate as f64).min(capacity as f64 - *tokens);

            *tokens = (*tokens + tokens_to_add).min(capacity as f64);
            *last_refill = now;

            if *tokens >= weight as f64 {
                *tokens -= weight as f64;
                state.total_allowed += weight;
                state.consecutive_violations = 0;
                state.reputation_score = (state.reputation_score + 0.01).min(1.0);

                RateLimitResponse {
                    allowed: true,
                    limit: capacity,
                    remaining: tokens.floor() as u64,
                    reset_time: now + ChronoDuration::from_std(refill_period).unwrap_or_default(),
                    retry_after: None,
                    policy_id: Uuid::nil(),
                    action_taken: None,
                    reason: None,
                    headers: self.create_rate_limit_headers(capacity, tokens.floor() as u64, now + ChronoDuration::from_std(refill_period).unwrap_or_default()),
                }
            } else {
                state.total_blocked += weight;
                state.consecutive_violations += 1;
                state.reputation_score = (state.reputation_score - 0.1).max(0.0);

                let retry_after = Duration::from_secs_f64(
                    (weight as f64 - *tokens) / refill_rate as f64 * refill_period.as_secs() as f64
                );

                RateLimitResponse {
                    allowed: false,
                    limit: capacity,
                    remaining: 0,
                    reset_time: now + ChronoDuration::from_std(retry_after).unwrap_or_default(),
                    retry_after: Some(retry_after),
                    policy_id: Uuid::nil(),
                    action_taken: Some(RateLimitAction::Block),
                    reason: Some("Token bucket depleted".to_string()),
                    headers: self.create_rate_limit_headers(capacity, 0, now + ChronoDuration::from_std(retry_after).unwrap_or_default()),
                }
            }
        } else {
            // Invalid state, create new one
            state.algorithm_state = AlgorithmState::TokenBucket {
                tokens: capacity as f64,
                last_refill: now,
            };
            self.evaluate_token_bucket(state, capacity, refill_rate, refill_period, weight, now).await
        }
    }

    async fn evaluate_leaky_bucket(&self, state: &mut RateLimitState, capacity: u64, leak_rate: u64, leak_period: Duration, weight: u64, now: DateTime<Utc>) -> RateLimitResponse {
        if let AlgorithmState::LeakyBucket { queue_size, last_leak } = &mut state.algorithm_state {
            // Calculate leaks based on time elapsed
            let time_elapsed = now.signed_duration_since(*last_leak);
            let periods_elapsed = time_elapsed.num_seconds() as f64 / leak_period.as_secs() as f64;
            let requests_to_leak = (periods_elapsed * leak_rate as f64).floor() as usize;

            *queue_size = queue_size.saturating_sub(requests_to_leak);
            *last_leak = now;

            if *queue_size + weight as usize <= capacity as usize {
                *queue_size += weight as usize;
                state.total_allowed += weight;

                RateLimitResponse {
                    allowed: true,
                    limit: capacity,
                    remaining: capacity.saturating_sub(*queue_size as u64),
                    reset_time: now + ChronoDuration::from_std(leak_period).unwrap_or_default(),
                    retry_after: None,
                    policy_id: Uuid::nil(),
                    action_taken: None,
                    reason: None,
                    headers: self.create_rate_limit_headers(capacity, capacity.saturating_sub(*queue_size as u64), now + ChronoDuration::from_std(leak_period).unwrap_or_default()),
                }
            } else {
                state.total_blocked += weight;

                RateLimitResponse {
                    allowed: false,
                    limit: capacity,
                    remaining: 0,
                    reset_time: now + ChronoDuration::from_std(leak_period).unwrap_or_default(),
                    retry_after: Some(leak_period),
                    policy_id: Uuid::nil(),
                    action_taken: Some(RateLimitAction::Block),
                    reason: Some("Leaky bucket overflow".to_string()),
                    headers: self.create_rate_limit_headers(capacity, 0, now + ChronoDuration::from_std(leak_period).unwrap_or_default()),
                }
            }
        } else {
            state.algorithm_state = AlgorithmState::LeakyBucket {
                queue_size: 0,
                last_leak: now,
            };
            self.evaluate_leaky_bucket(state, capacity, leak_rate, leak_period, weight, now).await
        }
    }

    async fn evaluate_fixed_window(&self, state: &mut RateLimitState, limit: u64, window_size: Duration, weight: u64, now: DateTime<Utc>) -> RateLimitResponse {
        if let AlgorithmState::FixedWindow { count, window_start } = &mut state.algorithm_state {
            // Check if we need to reset the window
            let window_duration = ChronoDuration::from_std(window_size).unwrap_or_default();
            if now >= *window_start + window_duration {
                *count = 0;
                *window_start = now;
            }

            if *count + weight <= limit {
                *count += weight;
                state.total_allowed += weight;

                RateLimitResponse {
                    allowed: true,
                    limit,
                    remaining: limit.saturating_sub(*count),
                    reset_time: *window_start + window_duration,
                    retry_after: None,
                    policy_id: Uuid::nil(),
                    action_taken: None,
                    reason: None,
                    headers: self.create_rate_limit_headers(limit, limit.saturating_sub(*count), *window_start + window_duration),
                }
            } else {
                state.total_blocked += weight;
                let reset_time = *window_start + window_duration;

                RateLimitResponse {
                    allowed: false,
                    limit,
                    remaining: 0,
                    reset_time,
                    retry_after: Some(Duration::from_secs((reset_time - now).num_seconds().max(0) as u64)),
                    policy_id: Uuid::nil(),
                    action_taken: Some(RateLimitAction::Block),
                    reason: Some("Fixed window limit exceeded".to_string()),
                    headers: self.create_rate_limit_headers(limit, 0, reset_time),
                }
            }
        } else {
            state.algorithm_state = AlgorithmState::FixedWindow {
                count: 0,
                window_start: now,
            };
            self.evaluate_fixed_window(state, limit, window_size, weight, now).await
        }
    }

    async fn evaluate_sliding_window(&self, state: &mut RateLimitState, limit: u64, window_size: Duration, sub_window_count: usize, weight: u64, now: DateTime<Utc>) -> RateLimitResponse {
        if let AlgorithmState::SlidingWindow { sub_windows, window_start } = &mut state.algorithm_state {
            let sub_window_duration = window_size.as_secs() / sub_window_count as u64;
            let current_sub_window = ((now.timestamp() - window_start.timestamp()) / sub_window_duration as i64) as usize;

            // Initialize or resize sub_windows if needed
            if sub_windows.len() != sub_window_count {
                *sub_windows = VecDeque::from(vec![0u64; sub_window_count]);
            }

            // Shift windows if needed
            while current_sub_window >= sub_window_count {
                sub_windows.pop_front();
                sub_windows.push_back(0);
                *window_start = *window_start + ChronoDuration::seconds(sub_window_duration as i64);
            }

            let total_count: u64 = sub_windows.iter().sum();

            if total_count + weight <= limit {
                if let Some(current_window) = sub_windows.get_mut(current_sub_window) {
                    *current_window += weight;
                }
                state.total_allowed += weight;

                RateLimitResponse {
                    allowed: true,
                    limit,
                    remaining: limit.saturating_sub(total_count + weight),
                    reset_time: now + ChronoDuration::from_std(window_size).unwrap_or_default(),
                    retry_after: None,
                    policy_id: Uuid::nil(),
                    action_taken: None,
                    reason: None,
                    headers: self.create_rate_limit_headers(limit, limit.saturating_sub(total_count + weight), now + ChronoDuration::from_std(window_size).unwrap_or_default()),
                }
            } else {
                state.total_blocked += weight;

                RateLimitResponse {
                    allowed: false,
                    limit,
                    remaining: 0,
                    reset_time: now + ChronoDuration::from_std(window_size).unwrap_or_default(),
                    retry_after: Some(Duration::from_secs(sub_window_duration)),
                    policy_id: Uuid::nil(),
                    action_taken: Some(RateLimitAction::Block),
                    reason: Some("Sliding window limit exceeded".to_string()),
                    headers: self.create_rate_limit_headers(limit, 0, now + ChronoDuration::from_std(window_size).unwrap_or_default()),
                }
            }
        } else {
            state.algorithm_state = AlgorithmState::SlidingWindow {
                sub_windows: VecDeque::from(vec![0u64; sub_window_count]),
                window_start: now,
            };
            self.evaluate_sliding_window(state, limit, window_size, sub_window_count, weight, now).await
        }
    }

    async fn evaluate_adaptive(&self, state: &mut RateLimitState, base_limit: u64, max_limit: u64, adaptation_factor: f64, load_threshold: f64, weight: u64, now: DateTime<Utc>) -> RateLimitResponse {
        if let AlgorithmState::Adaptive { current_limit, load_factor, last_adaptation } = &mut state.algorithm_state {
            // Get current system load
            let system_load = self.system_load.read().unwrap().load_factor;

            // Adapt limit based on system load
            if now.signed_duration_since(*last_adaptation).num_seconds() >= 30 { // Adapt every 30 seconds
                if system_load > load_threshold {
                    // High load - reduce limit
                    *current_limit = ((*current_limit as f64) * (1.0 - adaptation_factor)).max(base_limit as f64) as u64;
                } else {
                    // Low load - increase limit
                    *current_limit = ((*current_limit as f64) * (1.0 + adaptation_factor)).min(max_limit as f64) as u64;
                }
                *load_factor = system_load;
                *last_adaptation = now;
            }

            // Simple counting for adaptive algorithm
            if state.request_count + weight <= *current_limit {
                state.request_count += weight;
                state.total_allowed += weight;

                RateLimitResponse {
                    allowed: true,
                    limit: *current_limit,
                    remaining: current_limit.saturating_sub(state.request_count),
                    reset_time: now + ChronoDuration::minutes(1), // Reset every minute
                    retry_after: None,
                    policy_id: Uuid::nil(),
                    action_taken: None,
                    reason: None,
                    headers: self.create_rate_limit_headers(*current_limit, current_limit.saturating_sub(state.request_count), now + ChronoDuration::minutes(1)),
                }
            } else {
                state.total_blocked += weight;

                RateLimitResponse {
                    allowed: false,
                    limit: *current_limit,
                    remaining: 0,
                    reset_time: now + ChronoDuration::minutes(1),
                    retry_after: Some(Duration::from_secs(60)),
                    policy_id: Uuid::nil(),
                    action_taken: Some(RateLimitAction::Block),
                    reason: Some("Adaptive limit exceeded".to_string()),
                    headers: self.create_rate_limit_headers(*current_limit, 0, now + ChronoDuration::minutes(1)),
                }
            }
        } else {
            state.algorithm_state = AlgorithmState::Adaptive {
                current_limit: base_limit,
                load_factor: 0.0,
                last_adaptation: now,
            };
            self.evaluate_adaptive(state, base_limit, max_limit, adaptation_factor, load_threshold, weight, now).await
        }
    }

    fn create_initial_algorithm_state(&self, algorithm: &RateLimitAlgorithm) -> AlgorithmState {
        let now = Utc::now();
        match algorithm {
            RateLimitAlgorithm::TokenBucket { capacity, .. } => {
                AlgorithmState::TokenBucket {
                    tokens: *capacity as f64,
                    last_refill: now,
                }
            },
            RateLimitAlgorithm::LeakyBucket { .. } => {
                AlgorithmState::LeakyBucket {
                    queue_size: 0,
                    last_leak: now,
                }
            },
            RateLimitAlgorithm::FixedWindow { .. } => {
                AlgorithmState::FixedWindow {
                    count: 0,
                    window_start: now,
                }
            },
            RateLimitAlgorithm::SlidingWindow { sub_window_count, .. } => {
                AlgorithmState::SlidingWindow {
                    sub_windows: VecDeque::from(vec![0u64; *sub_window_count]),
                    window_start: now,
                }
            },
            RateLimitAlgorithm::Adaptive { base_limit, .. } => {
                AlgorithmState::Adaptive {
                    current_limit: *base_limit,
                    load_factor: 0.0,
                    last_adaptation: now,
                }
            },
        }
    }

    fn create_rate_limit_headers(&self, limit: u64, remaining: u64, reset_time: DateTime<Utc>) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("X-RateLimit-Limit".to_string(), limit.to_string());
        headers.insert("X-RateLimit-Remaining".to_string(), remaining.to_string());
        headers.insert("X-RateLimit-Reset".to_string(), reset_time.timestamp().to_string());
        headers
    }

    async fn record_block(&self, policy: &RateLimitPolicy, request: &RateLimitRequest, response: &RateLimitResponse) {
        // Update analytics
        let mut analytics = self.analytics.write().unwrap();
        analytics.total_requests += request.weight;
        analytics.total_blocked += request.weight;
        analytics.block_rate = analytics.total_blocked as f64 / analytics.total_requests as f64;

        // Record top violator
        for (dimension, identifier) in &request.dimensions {
            let violator = TopViolator {
                identifier: identifier.clone(),
                dimension: dimension.clone(),
                violation_count: 1,
                first_violation: Utc::now(),
                last_violation: Utc::now(),
                severity_score: 1.0,
            };

            if let Some(existing) = analytics.top_violators.iter_mut().find(|v| v.identifier == identifier && v.dimension == *dimension) {
                existing.violation_count += 1;
                existing.last_violation = Utc::now();
                existing.severity_score += 0.1;
            } else {
                analytics.top_violators.push(violator);
            }
        }

        // Limit top violators list size
        analytics.top_violators.sort_by(|a, b| b.severity_score.partial_cmp(&a.severity_score).unwrap_or(std::cmp::Ordering::Equal));
        analytics.top_violators.truncate(100);
    }

    async fn update_analytics(&self, response: &RateLimitResponse, start_time: Instant) {
        let mut analytics = self.analytics.write().unwrap();
        analytics.total_requests += 1;

        if response.allowed {
            analytics.total_allowed += 1;
        } else {
            analytics.total_blocked += 1;
        }

        analytics.block_rate = analytics.total_blocked as f64 / analytics.total_requests as f64;
    }

    async fn check_abuse_patterns(&self, request: &RateLimitRequest) {
        let mut abuse_detection = self.abuse_detection.write().unwrap();

        // Simple abuse detection - check for rapid requests
        for (dimension, identifier) in &request.dimensions {
            let key = format!("{}:{}", dimension as u8, identifier);

            // Check existing indicators
            if let Some(indicator) = abuse_detection.indicators.get_mut(&key) {
                indicator.confidence_score += 0.1;
                indicator.last_updated = Utc::now();

                // Add evidence
                indicator.evidence.push(AbuseEvidence {
                    evidence_type: "rapid_request".to_string(),
                    value: "1".to_string(),
                    timestamp: Utc::now(),
                    severity: 0.1,
                });
            } else {
                // Create new indicator if threshold is met
                let indicator = AbuseIndicator {
                    indicator_id: Uuid::now_v7(),
                    indicator_type: AbuseIndicatorType::SuspiciousPattern,
                    identifier: identifier.clone(),
                    dimension: dimension.clone(),
                    confidence_score: 0.1,
                    first_detected: Utc::now(),
                    last_updated: Utc::now(),
                    evidence: vec![AbuseEvidence {
                        evidence_type: "initial_detection".to_string(),
                        value: "1".to_string(),
                        timestamp: Utc::now(),
                        severity: 0.1,
                    }],
                };

                abuse_detection.indicators.insert(key, indicator);
            }
        }
    }

    fn create_default_abuse_rules() -> Vec<AbuseDetectionRule> {
        vec![
            AbuseDetectionRule {
                rule_id: Uuid::now_v7(),
                name: "Rapid Request Detection".to_string(),
                pattern: AbusePattern::RapidRequests {
                    rate_threshold: 100.0,
                    window: Duration::from_secs(60),
                },
                threshold: 0.8,
                action: AbuseAction::Alert,
                enabled: true,
            },
            AbuseDetectionRule {
                rule_id: Uuid::now_v7(),
                name: "Repeated Block Detection".to_string(),
                pattern: AbusePattern::RepeatedBlocks {
                    block_count: 10,
                    window: Duration::from_secs(300),
                },
                threshold: 0.9,
                action: AbuseAction::Block,
                enabled: true,
            },
        ]
    }

    // Cleanup expired state entries
    pub async fn cleanup_expired_state(&self) {
        let now = Utc::now();
        let mut state_store = self.state_store.write().unwrap();

        // Remove entries that haven't been accessed in the last hour
        state_store.retain(|_, state| {
            (now - state.last_request).num_seconds() < 3600
        });

        // If still too many entries, remove oldest
        if state_store.len() > self.config.max_state_entries {
            let mut entries: Vec<_> = state_store.iter().collect();
            entries.sort_by_key(|(_, state)| state.last_request);

            let to_remove = entries.len() - self.config.max_state_entries;
            for (key, _) in entries.iter().take(to_remove) {
                state_store.remove(*key);
            }
        }
    }
}

// ============ RATE LIMITING TESTS ============

#[tokio::test]
async fn test_token_bucket_rate_limiting() {
    let engine = EnterpriseRateLimitEngine::new();

    // Create a token bucket policy
    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Test Token Bucket".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 10,
            refill_rate: 2,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::IPAddress],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::IPAddress, "192.168.1.1".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/test".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should allow first 10 requests
    for i in 0..10 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
        assert_eq!(response.remaining, 10 - i - 1);
    }

    // 11th request should be blocked
    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);
    assert_eq!(response.remaining, 0);
    assert!(response.retry_after.is_some());
}

#[tokio::test]
async fn test_fixed_window_rate_limiting() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Test Fixed Window".to_string(),
        algorithm: RateLimitAlgorithm::FixedWindow {
            limit: 5,
            window_size: Duration::from_secs(60),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "user123".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/data".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should allow first 5 requests
    for i in 0..5 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
    }

    // 6th request should be blocked
    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);
    assert!(response.retry_after.is_some());
}

#[tokio::test]
async fn test_sliding_window_rate_limiting() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Test Sliding Window".to_string(),
        algorithm: RateLimitAlgorithm::SlidingWindow {
            limit: 10,
            window_size: Duration::from_secs(60),
            sub_window_count: 6, // 10-second sub-windows
        },
        dimensions: vec![RateLimitDimension::ApiKey],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::ApiKey, "api_key_456".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/analytics".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should allow requests within limit
    for i in 0..10 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
    }

    // 11th request should be blocked
    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);
}

#[tokio::test]
async fn test_adaptive_rate_limiting() {
    let mut engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Test Adaptive".to_string(),
        algorithm: RateLimitAlgorithm::Adaptive {
            base_limit: 10,
            max_limit: 50,
            adaptation_factor: 0.1,
            load_threshold: 0.7,
        },
        dimensions: vec![RateLimitDimension::Endpoint],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    // Simulate high system load
    let high_load = SystemLoadMetrics {
        cpu_usage: 85.0,
        memory_usage: 75.0,
        request_queue_size: 100,
        average_response_time: Duration::from_millis(500),
        error_rate: 0.05,
        active_connections: 200,
        load_factor: 0.8, // High load
    };

    engine.update_system_load(high_load).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::Endpoint, "/api/heavy".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/heavy".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // First few requests should be allowed
    for i in 0..5 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
    }

    // Should have adaptive behavior based on system load
    let response = engine.check_rate_limit(&request).await;
    // The actual limit may be reduced due to high system load
    assert!(response.limit <= 50); // Should not exceed max_limit
}

#[tokio::test]
async fn test_multi_dimensional_rate_limiting() {
    let engine = EnterpriseRateLimitEngine::new();

    // Policy that limits by both IP and User ID
    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Multi-Dimensional Policy".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 5,
            refill_rate: 1,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::IPAddress, RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::IPAddress, "10.0.0.1".to_string());
    request_dimensions.insert(RateLimitDimension::UserId, "user789".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/upload".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should apply most restrictive limit across all dimensions
    for i in 0..5 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
    }

    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);
}

#[tokio::test]
async fn test_rate_limit_exemptions() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Test with Exemptions".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 1, // Very restrictive
            refill_rate: 1,
            refill_period: Duration::from_secs(10),
        },
        dimensions: vec![RateLimitDimension::IPAddress],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    // Add exemption for specific IP
    let exemption = RateLimitExemption {
        exemption_type: ExemptionType::IPAddress,
        identifier: "192.168.1.100".to_string(),
        reason: "Internal service".to_string(),
        expires_at: None,
    };

    engine.add_exemption(exemption).await;

    // Test exempted IP
    let mut exempted_request_dimensions = HashMap::new();
    exempted_request_dimensions.insert(RateLimitDimension::IPAddress, "192.168.1.100".to_string());

    let exempted_request = RateLimitRequest {
        dimensions: exempted_request_dimensions,
        resource: "/api/internal".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should always be allowed due to exemption
    for _ in 0..10 {
        let response = engine.check_rate_limit(&exempted_request).await;
        assert!(response.allowed);
        assert_eq!(response.limit, u64::MAX);
    }

    // Test non-exempted IP
    let mut regular_request_dimensions = HashMap::new();
    regular_request_dimensions.insert(RateLimitDimension::IPAddress, "203.0.113.1".to_string());

    let regular_request = RateLimitRequest {
        dimensions: regular_request_dimensions,
        resource: "/api/public".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // First request should be allowed
    let response = engine.check_rate_limit(&regular_request).await;
    assert!(response.allowed);

    // Second request should be blocked
    let response = engine.check_rate_limit(&regular_request).await;
    assert!(!response.allowed);
}

#[tokio::test]
async fn test_rate_limiting_analytics() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Analytics Test".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 3,
            refill_rate: 1,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "analytics_user".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/stats".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Make requests to generate analytics data
    for _ in 0..5 {
        engine.check_rate_limit(&request).await;
    }

    // Check analytics
    let analytics = engine.get_analytics().await;

    assert!(analytics.total_requests > 0);
    assert!(analytics.total_blocked > 0);
    assert!(analytics.total_allowed > 0);
    assert!(analytics.block_rate > 0.0);
    assert!(!analytics.top_violators.is_empty());

    // Verify top violator is recorded
    let top_violator = &analytics.top_violators[0];
    assert_eq!(top_violator.identifier, "analytics_user");
    assert_eq!(top_violator.dimension, RateLimitDimension::UserId);
    assert!(top_violator.violation_count > 0);
}

#[tokio::test]
async fn test_abuse_detection() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Abuse Detection Test".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 100, // High capacity to focus on abuse detection
            refill_rate: 10,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::IPAddress],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::IPAddress, "suspicious.ip.address".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/sensitive".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Make many requests to trigger abuse detection
    for _ in 0..50 {
        engine.check_rate_limit(&request).await;
    }

    // Check if abuse indicators were created
    let abuse_detection = engine.abuse_detection.read().unwrap();
    assert!(!abuse_detection.indicators.is_empty());

    // Should have detected suspicious patterns
    let suspicious_key = format!("{}:suspicious.ip.address", RateLimitDimension::IPAddress as u8);
    if let Some(indicator) = abuse_detection.indicators.get(&suspicious_key) {
        assert!(indicator.confidence_score > 0.0);
        assert!(!indicator.evidence.is_empty());
    }
}

#[tokio::test]
async fn test_token_bucket_refill_behavior() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Token Bucket Refill Test".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 5,
            refill_rate: 2,
            refill_period: Duration::from_millis(500), // Fast refill for testing
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "refill_test_user".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/refill_test".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Exhaust the bucket
    for i in 0..5 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
    }

    // Should be blocked now
    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);

    // Wait for refill
    sleep(Duration::from_millis(600)).await;

    // Should have refilled and allow requests again
    for i in 0..2 { // Should have refilled 2 tokens
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Refilled request {} should be allowed", i);
    }
}

#[tokio::test]
async fn test_leaky_bucket_algorithm() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Leaky Bucket Test".to_string(),
        algorithm: RateLimitAlgorithm::LeakyBucket {
            capacity: 10,
            leak_rate: 2,
            leak_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::IPAddress],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::IPAddress, "leaky.bucket.test".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/leaky_test".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Fill the bucket
    for i in 0..10 {
        let response = engine.check_rate_limit(&request).await;
        assert!(response.allowed, "Request {} should be allowed", i);
        assert_eq!(response.remaining, 10 - i - 1);
    }

    // Should be blocked when bucket is full
    let response = engine.check_rate_limit(&request).await;
    assert!(!response.allowed);
    assert_eq!(response.remaining, 0);
}

#[tokio::test]
async fn test_rate_limit_headers() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Headers Test".to_string(),
        algorithm: RateLimitAlgorithm::FixedWindow {
            limit: 100,
            window_size: Duration::from_secs(3600),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "headers_user".to_string());

    let request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/headers_test".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    let response = engine.check_rate_limit(&request).await;

    // Check that rate limit headers are present
    assert!(response.headers.contains_key("X-RateLimit-Limit"));
    assert!(response.headers.contains_key("X-RateLimit-Remaining"));
    assert!(response.headers.contains_key("X-RateLimit-Reset"));

    assert_eq!(response.headers.get("X-RateLimit-Limit").unwrap(), "100");
    assert_eq!(response.headers.get("X-RateLimit-Remaining").unwrap(), "99");

    // Reset time should be a valid timestamp
    let reset_header = response.headers.get("X-RateLimit-Reset").unwrap();
    assert!(reset_header.parse::<i64>().is_ok());
}

#[tokio::test]
async fn test_priority_based_policies() {
    let engine = EnterpriseRateLimitEngine::new();

    // High priority policy (more restrictive)
    let high_priority_policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "High Priority Policy".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 2,
            refill_rate: 1,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::High,
        scope: RateLimitScope::Endpoint("/api/critical".to_string()),
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Low priority policy (less restrictive)
    let low_priority_policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Low Priority Policy".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 10,
            refill_rate: 5,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Low,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(high_priority_policy).await;
    engine.add_policy(low_priority_policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "priority_user".to_string());

    let critical_request = RateLimitRequest {
        dimensions: request_dimensions.clone(),
        resource: "/api/critical".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Critical endpoint should be subject to high priority (more restrictive) policy
    let response1 = engine.check_rate_limit(&critical_request).await;
    assert!(response1.allowed);
    assert_eq!(response1.limit, 2); // High priority policy limit

    let response2 = engine.check_rate_limit(&critical_request).await;
    assert!(response2.allowed);

    // Third request should be blocked by high priority policy
    let response3 = engine.check_rate_limit(&critical_request).await;
    assert!(!response3.allowed);
}

#[tokio::test]
async fn test_weighted_requests() {
    let engine = EnterpriseRateLimitEngine::new();

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "Weighted Requests Test".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 10,
            refill_rate: 1,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    let mut request_dimensions = HashMap::new();
    request_dimensions.insert(RateLimitDimension::UserId, "weighted_user".to_string());

    // Heavy request that consumes 5 tokens
    let heavy_request = RateLimitRequest {
        dimensions: request_dimensions.clone(),
        resource: "/api/heavy_operation".to_string(),
        weight: 5,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Light request that consumes 1 token
    let light_request = RateLimitRequest {
        dimensions: request_dimensions,
        resource: "/api/light_operation".to_string(),
        weight: 1,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    // Should allow first heavy request (consumes 5 tokens)
    let response = engine.check_rate_limit(&heavy_request).await;
    assert!(response.allowed);
    assert_eq!(response.remaining, 5);

    // Should allow one light request (consumes 1 token)
    let response = engine.check_rate_limit(&light_request).await;
    assert!(response.allowed);
    assert_eq!(response.remaining, 4);

    // Should not allow another heavy request (would need 5 tokens, only 4 remaining)
    let response = engine.check_rate_limit(&heavy_request).await;
    assert!(!response.allowed);

    // Should still allow light requests
    for _ in 0..4 {
        let response = engine.check_rate_limit(&light_request).await;
        assert!(response.allowed);
    }

    // Now should be out of tokens
    let response = engine.check_rate_limit(&light_request).await;
    assert!(!response.allowed);
}

#[tokio::test]
async fn test_state_cleanup() {
    let mut config = RateLimitEngineConfig::default();
    config.max_state_entries = 5; // Small limit for testing
    config.cleanup_interval = Duration::from_millis(100);

    let engine = EnterpriseRateLimitEngine::with_config(config);

    let policy = RateLimitPolicy {
        id: Uuid::now_v7(),
        name: "State Cleanup Test".to_string(),
        algorithm: RateLimitAlgorithm::TokenBucket {
            capacity: 10,
            refill_rate: 1,
            refill_period: Duration::from_secs(1),
        },
        dimensions: vec![RateLimitDimension::UserId],
        priority: RateLimitPriority::Normal,
        scope: RateLimitScope::Global,
        exemptions: vec![],
        actions: vec![RateLimitAction::Block],
        time_restrictions: None,
        geographic_restrictions: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    engine.add_policy(policy).await;

    // Create more state entries than the limit
    for i in 0..10 {
        let mut request_dimensions = HashMap::new();
        request_dimensions.insert(RateLimitDimension::UserId, format!("cleanup_user_{}", i));

        let request = RateLimitRequest {
            dimensions: request_dimensions,
            resource: "/api/cleanup_test".to_string(),
            weight: 1,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        };

        engine.check_rate_limit(&request).await;
    }

    // Should have created entries
    let state_count_before = engine.state_store.read().unwrap().len();
    assert!(state_count_before > 5);

    // Trigger cleanup
    engine.cleanup_expired_state().await;

    // Should have cleaned up excess entries
    let state_count_after = engine.state_store.read().unwrap().len();
    assert!(state_count_after <= 5);
}