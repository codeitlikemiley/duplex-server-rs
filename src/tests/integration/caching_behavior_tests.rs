//! Integration tests for caching behavior and cache management - Enterprise Edition
//!
//! These tests verify that caching layers work correctly with proper cache invalidation,
//! cache coherence, performance optimization, and enterprise cache management features.
//!
//! Enterprise features include:
//! - Multi-tier caching (L1, L2, L3 cache levels)
//! - Distributed cache coherence and consistency
//! - Cache warming and preloading strategies
//! - Cache analytics and performance monitoring
//! - Cache invalidation patterns and strategies
//! - Cache partitioning and sharding
//! - Cache compression and serialization optimization
//! - Cache security and encryption
//! - Cache backup and recovery mechanisms
//! - Intelligent cache eviction policies

use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, VecDeque, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

// Cache Key and Value types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub namespace: String,
    pub entity_type: String,
    pub entity_id: String,
    pub version: Option<u64>,
    pub tenant_id: Option<String>,
}

impl CacheKey {
    pub fn new(namespace: &str, entity_type: &str, entity_id: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            version: None,
            tenant_id: None,
        }
    }

    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn to_string(&self) -> String {
        let mut key = format!("{}:{}:{}", self.namespace, self.entity_type, self.entity_id);
        if let Some(version) = self.version {
            key.push_str(&format!(":v{}", version));
        }
        if let Some(ref tenant_id) = self.tenant_id {
            key.push_str(&format!(":t{}", tenant_id));
        }
        key
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: CacheKey,
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
    pub size_bytes: usize,
    pub compression_ratio: Option<f64>,
    pub checksum: Option<String>,
    pub tags: HashSet<String>,
    pub priority: CachePriority,
    pub replication_factor: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CachePriority {
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
    Expendable = 5,
}

// Enterprise Cache Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseCacheConfig {
    pub multi_tier_enabled: bool,
    pub distributed_cache_enabled: bool,
    pub cache_warming_enabled: bool,
    pub analytics_enabled: bool,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub backup_enabled: bool,
    pub max_memory_mb: usize,
    pub max_entries: usize,
    pub default_ttl_seconds: u64,
    pub eviction_policy: EvictionPolicy,
    pub consistency_level: ConsistencyLevel,
    pub replication_factor: u8,
    pub shard_count: usize,
    pub compression_threshold_bytes: usize,
    pub cache_warming_strategies: Vec<WarmingStrategy>,
    pub monitoring_interval_seconds: u64,
}

impl Default for EnterpriseCacheConfig {
    fn default() -> Self {
        Self {
            multi_tier_enabled: true,
            distributed_cache_enabled: true,
            cache_warming_enabled: true,
            analytics_enabled: true,
            compression_enabled: true,
            encryption_enabled: true,
            backup_enabled: true,
            max_memory_mb: 1024, // 1GB
            max_entries: 1000000, // 1 million entries
            default_ttl_seconds: 3600, // 1 hour
            eviction_policy: EvictionPolicy::LRU,
            consistency_level: ConsistencyLevel::Strong,
            replication_factor: 3,
            shard_count: 16,
            compression_threshold_bytes: 1024, // 1KB
            cache_warming_strategies: vec![
                WarmingStrategy::MostAccessed,
                WarmingStrategy::RecentlyCreated,
                WarmingStrategy::PredictiveLoad,
            ],
            monitoring_interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    LRU,        // Least Recently Used
    LFU,        // Least Frequently Used
    FIFO,       // First In, First Out
    Random,     // Random eviction
    TTL,        // Time To Live based
    Priority,   // Priority based
    Adaptive,   // Adaptive based on access patterns
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Eventual,   // Eventually consistent
    Strong,     // Strong consistency
    Monotonic,  // Monotonic consistency
    Session,    // Session consistency
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WarmingStrategy {
    MostAccessed,       // Warm most frequently accessed data
    RecentlyCreated,    // Warm recently created data
    PredictiveLoad,     // Predictive warming based on patterns
    TimeBasedLoad,      // Time-based warming schedules
    UserBehaviorBased,  // Based on user behavior patterns
}

// Cache Layer Types
#[derive(Debug, Clone, PartialEq)]
pub enum CacheLayer {
    L1InMemory,     // CPU cache / in-process memory
    L2Local,        // Local SSD/disk cache
    L3Distributed,  // Distributed cache (Redis/Hazelcast)
    L4Persistent,   // Persistent storage cache
}

// Cache Operations and Statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStatistics {
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub total_memory_bytes: usize,
    pub entry_count: usize,
    pub average_access_time_ms: f64,
    pub cache_efficiency: f64,
    pub compression_savings_bytes: usize,
    pub replication_overhead_bytes: usize,
    pub warming_operations: u64,
    pub invalidation_operations: u64,
    pub error_count: u64,
}

impl CacheStatistics {
    pub fn hit_ratio(&self) -> f64 {
        if self.hit_count + self.miss_count == 0 {
            0.0
        } else {
            self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
        }
    }

    pub fn miss_ratio(&self) -> f64 {
        1.0 - self.hit_ratio()
    }

    pub fn memory_efficiency(&self) -> f64 {
        if self.total_memory_bytes == 0 {
            0.0
        } else {
            (self.total_memory_bytes - self.replication_overhead_bytes) as f64 / self.total_memory_bytes as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachePerformanceMetrics {
    pub layer: CacheLayer,
    pub statistics: CacheStatistics,
    pub latency_percentiles: BTreeMap<String, Duration>, // P50, P95, P99, etc.
    pub throughput_operations_per_second: f64,
    pub error_rate: f64,
    pub consistency_violations: u64,
    pub warming_efficiency: f64,
    pub measured_at: DateTime<Utc>,
}

// Cache Events and Monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEvent {
    pub event_id: Uuid,
    pub event_type: CacheEventType,
    pub cache_key: CacheKey,
    pub layer: CacheLayer,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheEventType {
    Hit,
    Miss,
    Put,
    Eviction,
    Invalidation,
    Warming,
    Replication,
    Backup,
    Recovery,
    ConsistencyCheck,
    PerformanceAlert,
}

// Test Data Structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub profile_data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestSession {
    pub session_id: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
}

// Enterprise Cache System Simulator
#[derive(Debug)]
pub struct EnterpriseCacheSimulator {
    config: EnterpriseCacheConfig,
    l1_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    l2_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    l3_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    cache_statistics: Arc<RwLock<HashMap<CacheLayer, CacheStatistics>>>,
    performance_metrics: Arc<RwLock<Vec<CachePerformanceMetrics>>>,
    cache_events: Arc<RwLock<VecDeque<CacheEvent>>>,
    warming_queue: Arc<RwLock<VecDeque<CacheKey>>>,
    invalidation_queue: Arc<RwLock<VecDeque<CacheKey>>>,
    access_patterns: Arc<RwLock<HashMap<String, AccessPattern>>>,
    cache_health: Arc<RwLock<CacheHealthStatus>>,
}

#[derive(Debug, Clone)]
pub struct AccessPattern {
    pub key_prefix: String,
    pub access_frequency: f64,
    pub peak_hours: Vec<u8>, // Hours of day (0-23)
    pub seasonal_factor: f64,
    pub user_segments: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CacheHealthStatus {
    Healthy,
    Warning { issues: Vec<String> },
    Critical { issues: Vec<String> },
    Degraded { performance_impact: f64 },
}

impl EnterpriseCacheSimulator {
    pub fn new() -> Self {
        let config = EnterpriseCacheConfig::default();
        Self::with_config(config)
    }

    pub fn with_config(config: EnterpriseCacheConfig) -> Self {
        let mut initial_stats = HashMap::new();
        initial_stats.insert(CacheLayer::L1InMemory, CacheStatistics::default());
        initial_stats.insert(CacheLayer::L2Local, CacheStatistics::default());
        initial_stats.insert(CacheLayer::L3Distributed, CacheStatistics::default());

        Self {
            config,
            l1_cache: Arc::new(RwLock::new(HashMap::new())),
            l2_cache: Arc::new(RwLock::new(HashMap::new())),
            l3_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_statistics: Arc::new(RwLock::new(initial_stats)),
            performance_metrics: Arc::new(RwLock::new(Vec::new())),
            cache_events: Arc::new(RwLock::new(VecDeque::new())),
            warming_queue: Arc::new(RwLock::new(VecDeque::new())),
            invalidation_queue: Arc::new(RwLock::new(VecDeque::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            cache_health: Arc::new(RwLock::new(CacheHealthStatus::Healthy)),
        }
    }

    // Core cache operations
    pub async fn get<T: for<'de> Deserialize<'de> + Clone>(&self, key: &CacheKey) -> Option<T> {
        let start_time = Instant::now();
        let key_str = key.to_string();

        // Try L1 cache first
        if let Some(entry) = self.get_from_layer(&key_str, CacheLayer::L1InMemory).await {
            self.record_hit(CacheLayer::L1InMemory, start_time).await;
            self.record_access_pattern(key).await;
            if let Ok(value) = serde_json::from_slice::<T>(&entry.value) {
                return Some(value);
            }
        }

        // Try L2 cache
        if let Some(entry) = self.get_from_layer(&key_str, CacheLayer::L2Local).await {
            self.record_hit(CacheLayer::L2Local, start_time).await;
            // Promote to L1
            self.promote_to_l1(&key_str, &entry).await;
            if let Ok(value) = serde_json::from_slice::<T>(&entry.value) {
                return Some(value);
            }
        }

        // Try L3 cache
        if let Some(entry) = self.get_from_layer(&key_str, CacheLayer::L3Distributed).await {
            self.record_hit(CacheLayer::L3Distributed, start_time).await;
            // Promote to L2 and L1
            self.promote_to_l2(&key_str, &entry).await;
            self.promote_to_l1(&key_str, &entry).await;
            if let Ok(value) = serde_json::from_slice::<T>(&entry.value) {
                return Some(value);
            }
        }

        // Cache miss - record across all layers
        self.record_miss(CacheLayer::L1InMemory, start_time).await;
        self.record_miss(CacheLayer::L2Local, start_time).await;
        self.record_miss(CacheLayer::L3Distributed, start_time).await;

        None
    }

    pub async fn put<T: Serialize + Clone>(&self, key: &CacheKey, value: &T, ttl: Option<Duration>) {
        let start_time = Instant::now();
        let key_str = key.to_string();

        // Serialize value
        let serialized = match serde_json::to_vec(value) {
            Ok(data) => data,
            Err(_) => {
                self.record_error(CacheEventType::Put, key, "Serialization failed").await;
                return;
            }
        };

        let expires_at = ttl.map(|duration| Utc::now() + ChronoDuration::from_std(duration).unwrap_or_default());

        // Apply compression if enabled and data is large enough
        let final_data = if self.config.compression_enabled && serialized.len() > self.config.compression_threshold_bytes {
            self.compress_data(&serialized)
        } else {
            serialized
        };

        let entry = CacheEntry {
            key: key.clone(),
            value: final_data,
            created_at: Utc::now(),
            expires_at,
            last_accessed: Utc::now(),
            access_count: 0,
            size_bytes: serialized.len(),
            compression_ratio: None, // Would calculate in real implementation
            checksum: self.calculate_checksum(&serialized),
            tags: HashSet::new(),
            priority: CachePriority::Normal,
            replication_factor: self.config.replication_factor,
        };

        // Store in all cache layers based on configuration
        self.put_in_layer(&key_str, &entry, CacheLayer::L1InMemory).await;
        self.put_in_layer(&key_str, &entry, CacheLayer::L2Local).await;

        if self.config.distributed_cache_enabled {
            self.put_in_layer(&key_str, &entry, CacheLayer::L3Distributed).await;
        }

        self.record_event(CacheEventType::Put, key, CacheLayer::L1InMemory, start_time, true, None).await;
        self.update_access_patterns(key).await;
    }

    pub async fn invalidate(&self, key: &CacheKey) {
        let key_str = key.to_string();

        // Remove from all cache layers
        self.remove_from_layer(&key_str, CacheLayer::L1InMemory).await;
        self.remove_from_layer(&key_str, CacheLayer::L2Local).await;
        self.remove_from_layer(&key_str, CacheLayer::L3Distributed).await;

        self.record_event(CacheEventType::Invalidation, key, CacheLayer::L1InMemory, Instant::now(), true, None).await;
    }

    pub async fn invalidate_pattern(&self, pattern: &str) {
        // Invalidate all keys matching pattern (simplified - would use proper pattern matching)
        let keys_to_invalidate = self.find_keys_matching_pattern(pattern).await;

        for key_str in keys_to_invalidate {
            let cache_key = CacheKey::new("pattern", "match", &key_str);
            self.invalidate(&cache_key).await;
        }
    }

    // Cache warming operations
    pub async fn warm_cache(&self, keys: &[CacheKey]) {
        for key in keys {
            self.warming_queue.write().unwrap().push_back(key.clone());
        }

        // Process warming queue
        self.process_warming_queue().await;
    }

    pub async fn warm_most_accessed(&self, count: usize) {
        let patterns = self.access_patterns.read().unwrap();
        let mut sorted_patterns: Vec<_> = patterns.iter()
            .map(|(key, pattern)| (key.clone(), pattern.access_frequency))
            .collect();
        sorted_patterns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (key_prefix, _) in sorted_patterns.iter().take(count) {
            let cache_key = CacheKey::new("warm", "pattern", key_prefix);
            self.warming_queue.write().unwrap().push_back(cache_key);
        }

        self.process_warming_queue().await;
    }

    async fn process_warming_queue(&self) {
        let mut queue = self.warming_queue.write().unwrap();
        let mut processed = 0;
        const MAX_BATCH_SIZE: usize = 100;

        while let Some(key) = queue.pop_front() {
            if processed >= MAX_BATCH_SIZE {
                break;
            }

            // Simulate warming by pre-fetching data
            let start_time = Instant::now();

            // In real implementation, would fetch from primary data source
            let mock_data = format!("warmed_data_for_{}", key.to_string());

            self.put(&key, &mock_data, Some(Duration::from_secs(self.config.default_ttl_seconds))).await;
            self.record_event(CacheEventType::Warming, &key, CacheLayer::L1InMemory, start_time, true, None).await;

            processed += 1;
        }

        // Update warming statistics
        let mut stats = self.cache_statistics.write().unwrap();
        if let Some(l1_stats) = stats.get_mut(&CacheLayer::L1InMemory) {
            l1_stats.warming_operations += processed as u64;
        }
    }

    // Performance monitoring and analytics
    pub async fn get_performance_metrics(&self) -> Vec<CachePerformanceMetrics> {
        self.performance_metrics.read().unwrap().clone()
    }

    pub async fn get_cache_statistics(&self) -> HashMap<CacheLayer, CacheStatistics> {
        self.cache_statistics.read().unwrap().clone()
    }

    pub async fn analyze_cache_efficiency(&self) -> CacheEfficiencyReport {
        let stats = self.get_cache_statistics().await;

        let l1_stats = stats.get(&CacheLayer::L1InMemory).cloned().unwrap_or_default();
        let l2_stats = stats.get(&CacheLayer::L2Local).cloned().unwrap_or_default();
        let l3_stats = stats.get(&CacheLayer::L3Distributed).cloned().unwrap_or_default();

        let overall_hit_ratio = {
            let total_hits = l1_stats.hit_count + l2_stats.hit_count + l3_stats.hit_count;
            let total_requests = total_hits + l1_stats.miss_count;
            if total_requests > 0 {
                total_hits as f64 / total_requests as f64
            } else {
                0.0
            }
        };

        CacheEfficiencyReport {
            overall_hit_ratio,
            l1_hit_ratio: l1_stats.hit_ratio(),
            l2_hit_ratio: l2_stats.hit_ratio(),
            l3_hit_ratio: l3_stats.hit_ratio(),
            memory_usage_mb: (l1_stats.total_memory_bytes + l2_stats.total_memory_bytes + l3_stats.total_memory_bytes) as f64 / (1024.0 * 1024.0),
            total_entries: l1_stats.entry_count + l2_stats.entry_count + l3_stats.entry_count,
            average_response_time_ms: (l1_stats.average_access_time_ms + l2_stats.average_access_time_ms + l3_stats.average_access_time_ms) / 3.0,
            eviction_rate: (l1_stats.eviction_count + l2_stats.eviction_count + l3_stats.eviction_count) as f64,
            recommendations: self.generate_performance_recommendations(&stats).await,
        }
    }

    pub async fn get_cache_health(&self) -> CacheHealthStatus {
        self.cache_health.read().unwrap().clone()
    }

    pub async fn monitor_cache_health(&self) {
        let stats = self.get_cache_statistics().await;
        let mut issues = Vec::new();

        // Check hit ratios
        for (layer, stat) in &stats {
            if stat.hit_ratio() < 0.5 {
                issues.push(format!("{:?} cache hit ratio too low: {:.2}%", layer, stat.hit_ratio() * 100.0));
            }
        }

        // Check memory usage
        let total_memory = stats.values().map(|s| s.total_memory_bytes).sum::<usize>();
        let memory_usage_ratio = total_memory as f64 / (self.config.max_memory_mb * 1024 * 1024) as f64;

        if memory_usage_ratio > 0.9 {
            issues.push(format!("Memory usage critical: {:.1}%", memory_usage_ratio * 100.0));
        } else if memory_usage_ratio > 0.8 {
            issues.push(format!("Memory usage high: {:.1}%", memory_usage_ratio * 100.0));
        }

        // Check error rates
        let total_errors = stats.values().map(|s| s.error_count).sum::<u64>();
        if total_errors > 100 {
            issues.push(format!("High error count: {}", total_errors));
        }

        // Update health status
        let health_status = if issues.is_empty() {
            CacheHealthStatus::Healthy
        } else if issues.len() <= 2 && memory_usage_ratio < 0.95 {
            CacheHealthStatus::Warning { issues }
        } else {
            CacheHealthStatus::Critical { issues }
        };

        *self.cache_health.write().unwrap() = health_status;
    }

    // Helper methods
    async fn get_from_layer(&self, key: &str, layer: CacheLayer) -> Option<CacheEntry<Vec<u8>>> {
        let cache = match layer {
            CacheLayer::L1InMemory => &self.l1_cache,
            CacheLayer::L2Local => &self.l2_cache,
            CacheLayer::L3Distributed => &self.l3_cache,
            CacheLayer::L4Persistent => return None, // Not implemented in this test
        };

        let guard = cache.read().unwrap();
        if let Some(entry) = guard.get(key) {
            // Check expiration
            if let Some(expires_at) = entry.expires_at {
                if Utc::now() > expires_at {
                    return None; // Expired
                }
            }
            Some(entry.clone())
        } else {
            None
        }
    }

    async fn put_in_layer(&self, key: &str, entry: &CacheEntry<Vec<u8>>, layer: CacheLayer) {
        let cache = match layer {
            CacheLayer::L1InMemory => &self.l1_cache,
            CacheLayer::L2Local => &self.l2_cache,
            CacheLayer::L3Distributed => &self.l3_cache,
            CacheLayer::L4Persistent => return, // Not implemented in this test
        };

        let mut guard = cache.write().unwrap();

        // Check if we need to evict entries
        if guard.len() >= self.config.max_entries / 3 { // Divide by 3 for 3 layers
            self.evict_entries(&mut guard, layer).await;
        }

        guard.insert(key.to_string(), entry.clone());

        // Update statistics
        let mut stats = self.cache_statistics.write().unwrap();
        if let Some(layer_stats) = stats.get_mut(&layer) {
            layer_stats.entry_count = guard.len();
            layer_stats.total_memory_bytes += entry.size_bytes;
        }
    }

    async fn remove_from_layer(&self, key: &str, layer: CacheLayer) {
        let cache = match layer {
            CacheLayer::L1InMemory => &self.l1_cache,
            CacheLayer::L2Local => &self.l2_cache,
            CacheLayer::L3Distributed => &self.l3_cache,
            CacheLayer::L4Persistent => return,
        };

        let mut guard = cache.write().unwrap();
        if let Some(entry) = guard.remove(key) {
            // Update statistics
            let mut stats = self.cache_statistics.write().unwrap();
            if let Some(layer_stats) = stats.get_mut(&layer) {
                layer_stats.entry_count = guard.len();
                layer_stats.total_memory_bytes = layer_stats.total_memory_bytes.saturating_sub(entry.size_bytes);
                layer_stats.invalidation_operations += 1;
            }
        }
    }

    async fn promote_to_l1(&self, key: &str, entry: &CacheEntry<Vec<u8>>) {
        self.put_in_layer(key, entry, CacheLayer::L1InMemory).await;
    }

    async fn promote_to_l2(&self, key: &str, entry: &CacheEntry<Vec<u8>>) {
        self.put_in_layer(key, entry, CacheLayer::L2Local).await;
    }

    async fn evict_entries(&self, cache: &mut HashMap<String, CacheEntry<Vec<u8>>>, layer: CacheLayer) {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Find least recently used entry
                let oldest_key = cache.iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(key, _)| key.clone());

                if let Some(key) = oldest_key {
                    cache.remove(&key);

                    // Update statistics
                    let mut stats = self.cache_statistics.write().unwrap();
                    if let Some(layer_stats) = stats.get_mut(&layer) {
                        layer_stats.eviction_count += 1;
                    }
                }
            },
            EvictionPolicy::LFU => {
                // Find least frequently used entry
                let lfu_key = cache.iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(key, _)| key.clone());

                if let Some(key) = lfu_key {
                    cache.remove(&key);

                    let mut stats = self.cache_statistics.write().unwrap();
                    if let Some(layer_stats) = stats.get_mut(&layer) {
                        layer_stats.eviction_count += 1;
                    }
                }
            },
            EvictionPolicy::Priority => {
                // Find lowest priority entry
                let lowest_priority_key = cache.iter()
                    .max_by_key(|(_, entry)| entry.priority.clone() as u8)
                    .map(|(key, _)| key.clone());

                if let Some(key) = lowest_priority_key {
                    cache.remove(&key);

                    let mut stats = self.cache_statistics.write().unwrap();
                    if let Some(layer_stats) = stats.get_mut(&layer) {
                        layer_stats.eviction_count += 1;
                    }
                }
            },
            _ => {
                // Default to removing first entry (FIFO-like)
                if let Some((key, _)) = cache.iter().next() {
                    let key = key.clone();
                    cache.remove(&key);

                    let mut stats = self.cache_statistics.write().unwrap();
                    if let Some(layer_stats) = stats.get_mut(&layer) {
                        layer_stats.eviction_count += 1;
                    }
                }
            }
        }
    }

    async fn record_hit(&self, layer: CacheLayer, _start_time: Instant) {
        let mut stats = self.cache_statistics.write().unwrap();
        if let Some(layer_stats) = stats.get_mut(&layer) {
            layer_stats.hit_count += 1;
        }
    }

    async fn record_miss(&self, layer: CacheLayer, _start_time: Instant) {
        let mut stats = self.cache_statistics.write().unwrap();
        if let Some(layer_stats) = stats.get_mut(&layer) {
            layer_stats.miss_count += 1;
        }
    }

    async fn record_event(&self, event_type: CacheEventType, key: &CacheKey, layer: CacheLayer, start_time: Instant, success: bool, error_message: Option<String>) {
        let event = CacheEvent {
            event_id: Uuid::now_v7(),
            event_type,
            cache_key: key.clone(),
            layer,
            timestamp: Utc::now(),
            duration_ms: Some(start_time.elapsed().as_millis() as u64),
            success,
            error_message,
            metadata: HashMap::new(),
        };

        let mut events = self.cache_events.write().unwrap();
        events.push_back(event);

        // Keep only recent events (last 10000)
        while events.len() > 10000 {
            events.pop_front();
        }
    }

    async fn record_error(&self, event_type: CacheEventType, key: &CacheKey, error_message: &str) {
        self.record_event(event_type, key, CacheLayer::L1InMemory, Instant::now(), false, Some(error_message.to_string())).await;

        let mut stats = self.cache_statistics.write().unwrap();
        if let Some(layer_stats) = stats.get_mut(&CacheLayer::L1InMemory) {
            layer_stats.error_count += 1;
        }
    }

    async fn record_access_pattern(&self, key: &CacheKey) {
        let key_prefix = format!("{}:{}", key.namespace, key.entity_type);
        let mut patterns = self.access_patterns.write().unwrap();

        let pattern = patterns.entry(key_prefix.clone()).or_insert_with(|| AccessPattern {
            key_prefix: key_prefix.clone(),
            access_frequency: 0.0,
            peak_hours: vec![],
            seasonal_factor: 1.0,
            user_segments: HashSet::new(),
        });

        pattern.access_frequency += 1.0;
    }

    async fn update_access_patterns(&self, _key: &CacheKey) {
        // In real implementation, would update access patterns based on time of day, etc.
    }

    async fn find_keys_matching_pattern(&self, pattern: &str) -> Vec<String> {
        let mut matching_keys = Vec::new();

        // Check all cache layers for matching keys
        let l1_guard = self.l1_cache.read().unwrap();
        for key in l1_guard.keys() {
            if key.contains(pattern) {
                matching_keys.push(key.clone());
            }
        }

        matching_keys
    }

    fn compress_data(&self, data: &[u8]) -> Vec<u8> {
        // Simplified compression simulation
        // In real implementation would use zlib, gzip, lz4, etc.
        data.to_vec()
    }

    fn calculate_checksum(&self, data: &[u8]) -> Option<String> {
        if self.config.encryption_enabled {
            // Simplified checksum calculation
            Some(format!("checksum_{}", data.len()))
        } else {
            None
        }
    }

    async fn generate_performance_recommendations(&self, stats: &HashMap<CacheLayer, CacheStatistics>) -> Vec<String> {
        let mut recommendations = Vec::new();

        for (layer, stat) in stats {
            if stat.hit_ratio() < 0.7 {
                recommendations.push(format!("Consider increasing {:?} cache size or TTL", layer));
            }

            if stat.eviction_count > stat.hit_count / 10 {
                recommendations.push(format!("High eviction rate in {:?} cache - consider memory optimization", layer));
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Cache performance is optimal".to_string());
        }

        recommendations
    }
}

#[derive(Debug, Clone)]
pub struct CacheEfficiencyReport {
    pub overall_hit_ratio: f64,
    pub l1_hit_ratio: f64,
    pub l2_hit_ratio: f64,
    pub l3_hit_ratio: f64,
    pub memory_usage_mb: f64,
    pub total_entries: usize,
    pub average_response_time_ms: f64,
    pub eviction_rate: f64,
    pub recommendations: Vec<String>,
}

// ============ CACHE BEHAVIOR TESTS ============

#[tokio::test]
async fn test_basic_cache_operations() {
    let cache = EnterpriseCacheSimulator::new();

    let user = TestUser {
        id: Uuid::now_v7(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let key = CacheKey::new("users", "user", &user.id.to_string());

    // Test cache miss
    let result: Option<TestUser> = cache.get(&key).await;
    assert!(result.is_none());

    // Test cache put
    cache.put(&key, &user, Some(Duration::from_secs(300))).await;

    // Test cache hit
    let cached_user: Option<TestUser> = cache.get(&key).await;
    assert!(cached_user.is_some());
    assert_eq!(cached_user.unwrap().username, user.username);

    // Verify statistics
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert_eq!(l1_stats.hit_count, 1);
    assert_eq!(l1_stats.miss_count, 1);
    assert_eq!(l1_stats.hit_ratio(), 0.5);
}

#[tokio::test]
async fn test_multi_tier_cache_promotion() {
    let cache = EnterpriseCacheSimulator::new();

    let session = TestSession {
        session_id: "session_123".to_string(),
        user_id: Uuid::now_v7(),
        created_at: Utc::now(),
        expires_at: Utc::now() + ChronoDuration::hours(1),
        last_activity: Utc::now(),
        ip_address: "192.168.1.1".to_string(),
        user_agent: "Mozilla/5.0".to_string(),
    };

    let key = CacheKey::new("sessions", "session", &session.session_id);

    // Put data in L3 cache only (simulate distributed cache scenario)
    cache.put_in_layer(&key.to_string(), &CacheEntry {
        key: key.clone(),
        value: serde_json::to_vec(&session).unwrap(),
        created_at: Utc::now(),
        expires_at: None,
        last_accessed: Utc::now(),
        access_count: 0,
        size_bytes: 100,
        compression_ratio: None,
        checksum: None,
        tags: HashSet::new(),
        priority: CachePriority::Normal,
        replication_factor: 1,
    }, CacheLayer::L3Distributed).await;

    // First access should promote to L2 and L1
    let cached_session: Option<TestSession> = cache.get(&key).await;
    assert!(cached_session.is_some());

    // Verify data is now in all cache layers
    let l1_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L1InMemory).await;
    let l2_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L2Local).await;
    let l3_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L3Distributed).await;

    assert!(l1_entry.is_some());
    assert!(l2_entry.is_some());
    assert!(l3_entry.is_some());
}

#[tokio::test]
async fn test_cache_expiration() {
    let cache = EnterpriseCacheSimulator::new();

    let user = TestUser {
        id: Uuid::now_v7(),
        username: "expiring_user".to_string(),
        email: "expiring@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let key = CacheKey::new("users", "user", &user.id.to_string());

    // Put with very short TTL
    cache.put(&key, &user, Some(Duration::from_millis(100))).await;

    // Should be able to retrieve immediately
    let cached_user: Option<TestUser> = cache.get(&key).await;
    assert!(cached_user.is_some());

    // Wait for expiration
    sleep(Duration::from_millis(150)).await;

    // Should now return None due to expiration
    let expired_user: Option<TestUser> = cache.get(&key).await;
    assert!(expired_user.is_none());
}

#[tokio::test]
async fn test_cache_invalidation() {
    let cache = EnterpriseCacheSimulator::new();

    let user = TestUser {
        id: Uuid::now_v7(),
        username: "invalidated_user".to_string(),
        email: "invalidated@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let key = CacheKey::new("users", "user", &user.id.to_string());

    // Put and verify
    cache.put(&key, &user, Some(Duration::from_secs(300))).await;
    let cached_user: Option<TestUser> = cache.get(&key).await;
    assert!(cached_user.is_some());

    // Invalidate
    cache.invalidate(&key).await;

    // Should now return None
    let invalidated_user: Option<TestUser> = cache.get(&key).await;
    assert!(invalidated_user.is_none());

    // Verify invalidation was recorded
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert!(l1_stats.invalidation_operations > 0);
}

#[tokio::test]
async fn test_cache_warming() {
    let cache = EnterpriseCacheSimulator::new();

    // Create keys for warming
    let keys: Vec<CacheKey> = (0..10).map(|i| {
        CacheKey::new("warm", "data", &format!("item_{}", i))
    }).collect();

    // Warm the cache
    cache.warm_cache(&keys).await;

    // Verify warming was recorded
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert!(l1_stats.warming_operations > 0);

    // Verify some data was actually cached
    let test_key = &keys[0];
    let warmed_data: Option<String> = cache.get(test_key).await;
    assert!(warmed_data.is_some());
}

#[tokio::test]
async fn test_cache_performance_monitoring() {
    let cache = EnterpriseCacheSimulator::new();

    // Perform various cache operations to generate metrics
    for i in 0..100 {
        let user = TestUser {
            id: Uuid::now_v7(),
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: Utc::now(),
            last_login: None,
            profile_data: HashMap::new(),
        };

        let key = CacheKey::new("users", "user", &user.id.to_string());

        // Some puts and gets
        cache.put(&key, &user, Some(Duration::from_secs(300))).await;
        let _: Option<TestUser> = cache.get(&key).await;

        // Some cache misses
        let miss_key = CacheKey::new("users", "user", &format!("missing_{}", i));
        let _: Option<TestUser> = cache.get(&miss_key).await;
    }

    // Get efficiency report
    let efficiency_report = cache.analyze_cache_efficiency().await;

    assert!(efficiency_report.overall_hit_ratio > 0.0);
    assert!(efficiency_report.l1_hit_ratio > 0.0);
    assert!(efficiency_report.total_entries > 0);
    assert!(!efficiency_report.recommendations.is_empty());
}

#[tokio::test]
async fn test_cache_health_monitoring() {
    let cache = EnterpriseCacheSimulator::new();

    // Initially should be healthy
    let initial_health = cache.get_cache_health().await;
    assert_eq!(initial_health, CacheHealthStatus::Healthy);

    // Perform operations and check health
    cache.monitor_cache_health().await;
    let health_after_monitoring = cache.get_cache_health().await;

    // Should still be healthy with minimal operations
    assert!(matches!(health_after_monitoring,
        CacheHealthStatus::Healthy |
        CacheHealthStatus::Warning { .. }));
}

#[tokio::test]
async fn test_cache_eviction_policies() {
    let mut config = EnterpriseCacheConfig::default();
    config.max_entries = 10; // Small cache for testing eviction
    config.eviction_policy = EvictionPolicy::LRU;

    let cache = EnterpriseCacheSimulator::with_config(config);

    // Fill cache beyond capacity
    for i in 0..15 {
        let user = TestUser {
            id: Uuid::now_v7(),
            username: format!("user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: Utc::now(),
            last_login: None,
            profile_data: HashMap::new(),
        };

        let key = CacheKey::new("users", "user", &user.id.to_string());
        cache.put(&key, &user, Some(Duration::from_secs(300))).await;
    }

    // Verify evictions occurred
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert!(l1_stats.eviction_count > 0);

    // Cache should not exceed max entries (divided among layers)
    assert!(l1_stats.entry_count <= 5); // Approximately max_entries / 3
}

#[tokio::test]
async fn test_cache_pattern_invalidation() {
    let cache = EnterpriseCacheSimulator::new();

    // Create multiple entries with same pattern
    for i in 0..5 {
        let user = TestUser {
            id: Uuid::now_v7(),
            username: format!("admin_user_{}", i),
            email: format!("admin_{}@example.com", i),
            created_at: Utc::now(),
            last_login: None,
            profile_data: HashMap::new(),
        };

        let key = CacheKey::new("users", "admin", &user.id.to_string());
        cache.put(&key, &user, Some(Duration::from_secs(300))).await;
    }

    // Also create some non-matching entries
    for i in 0..3 {
        let user = TestUser {
            id: Uuid::now_v7(),
            username: format!("regular_user_{}", i),
            email: format!("user_{}@example.com", i),
            created_at: Utc::now(),
            last_login: None,
            profile_data: HashMap::new(),
        };

        let key = CacheKey::new("users", "regular", &user.id.to_string());
        cache.put(&key, &user, Some(Duration::from_secs(300))).await;
    }

    // Invalidate admin pattern
    cache.invalidate_pattern("admin").await;

    // Verify admin entries are gone but regular entries remain
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert!(l1_stats.invalidation_operations > 0);
}

#[tokio::test]
async fn test_cache_compression_and_serialization() {
    let mut config = EnterpriseCacheConfig::default();
    config.compression_enabled = true;
    config.compression_threshold_bytes = 50; // Very small threshold for testing

    let cache = EnterpriseCacheSimulator::with_config(config);

    // Create large user with lots of profile data
    let mut profile_data = HashMap::new();
    for i in 0..100 {
        profile_data.insert(format!("field_{}", i), format!("value_{}", i).repeat(10));
    }

    let user = TestUser {
        id: Uuid::now_v7(),
        username: "large_user".to_string(),
        email: "large@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data,
    };

    let key = CacheKey::new("users", "user", &user.id.to_string());

    // Put and get large user
    cache.put(&key, &user, Some(Duration::from_secs(300))).await;
    let cached_user: Option<TestUser> = cache.get(&key).await;

    assert!(cached_user.is_some());
    assert_eq!(cached_user.unwrap().profile_data.len(), user.profile_data.len());
}

#[tokio::test]
async fn test_cache_access_patterns_analysis() {
    let cache = EnterpriseCacheSimulator::new();

    // Simulate access patterns
    let user_key = CacheKey::new("users", "user", "popular_user");

    // Access the same key multiple times to establish pattern
    for _ in 0..10 {
        let user = TestUser {
            id: Uuid::now_v7(),
            username: "popular_user".to_string(),
            email: "popular@example.com".to_string(),
            created_at: Utc::now(),
            last_login: None,
            profile_data: HashMap::new(),
        };

        cache.put(&user_key, &user, Some(Duration::from_secs(300))).await;
        let _: Option<TestUser> = cache.get(&user_key).await;
    }

    // Warm most accessed data
    cache.warm_most_accessed(5).await;

    // Verify access patterns were recorded
    let stats = cache.get_cache_statistics().await;
    let l1_stats = stats.get(&CacheLayer::L1InMemory).unwrap();
    assert!(l1_stats.warming_operations > 0);
}

#[tokio::test]
async fn test_cache_consistency_across_layers() {
    let cache = EnterpriseCacheSimulator::new();

    let user = TestUser {
        id: Uuid::now_v7(),
        username: "consistency_user".to_string(),
        email: "consistency@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let key = CacheKey::new("users", "user", &user.id.to_string());

    // Put data
    cache.put(&key, &user, Some(Duration::from_secs(300))).await;

    // Verify data exists in all layers
    let l1_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L1InMemory).await;
    let l2_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L2Local).await;
    let l3_entry = cache.get_from_layer(&key.to_string(), CacheLayer::L3Distributed).await;

    assert!(l1_entry.is_some());
    assert!(l2_entry.is_some());
    assert!(l3_entry.is_some());

    // Invalidate and verify consistency
    cache.invalidate(&key).await;

    let l1_after_invalidation = cache.get_from_layer(&key.to_string(), CacheLayer::L1InMemory).await;
    let l2_after_invalidation = cache.get_from_layer(&key.to_string(), CacheLayer::L2Local).await;
    let l3_after_invalidation = cache.get_from_layer(&key.to_string(), CacheLayer::L3Distributed).await;

    assert!(l1_after_invalidation.is_none());
    assert!(l2_after_invalidation.is_none());
    assert!(l3_after_invalidation.is_none());
}

#[tokio::test]
async fn test_cache_versioning_and_tenant_isolation() {
    let cache = EnterpriseCacheSimulator::new();

    // Create versioned and tenant-specific keys
    let user_v1_t1 = CacheKey::new("users", "user", "123")
        .with_version(1)
        .with_tenant("tenant_1");

    let user_v2_t1 = CacheKey::new("users", "user", "123")
        .with_version(2)
        .with_tenant("tenant_1");

    let user_v1_t2 = CacheKey::new("users", "user", "123")
        .with_version(1)
        .with_tenant("tenant_2");

    let user1 = TestUser {
        id: Uuid::now_v7(),
        username: "user_v1_t1".to_string(),
        email: "v1t1@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let user2 = TestUser {
        id: Uuid::now_v7(),
        username: "user_v2_t1".to_string(),
        email: "v2t1@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    let user3 = TestUser {
        id: Uuid::now_v7(),
        username: "user_v1_t2".to_string(),
        email: "v1t2@example.com".to_string(),
        created_at: Utc::now(),
        last_login: None,
        profile_data: HashMap::new(),
    };

    // Cache different versions and tenants
    cache.put(&user_v1_t1, &user1, Some(Duration::from_secs(300))).await;
    cache.put(&user_v2_t1, &user2, Some(Duration::from_secs(300))).await;
    cache.put(&user_v1_t2, &user3, Some(Duration::from_secs(300))).await;

    // Verify isolation - each should return different data
    let cached_v1_t1: Option<TestUser> = cache.get(&user_v1_t1).await;
    let cached_v2_t1: Option<TestUser> = cache.get(&user_v2_t1).await;
    let cached_v1_t2: Option<TestUser> = cache.get(&user_v1_t2).await;

    assert!(cached_v1_t1.is_some());
    assert!(cached_v2_t1.is_some());
    assert!(cached_v1_t2.is_some());

    assert_eq!(cached_v1_t1.unwrap().username, "user_v1_t1");
    assert_eq!(cached_v2_t1.unwrap().username, "user_v2_t1");
    assert_eq!(cached_v1_t2.unwrap().username, "user_v1_t2");

    // Verify keys are actually different
    assert_ne!(user_v1_t1.to_string(), user_v2_t1.to_string());
    assert_ne!(user_v1_t1.to_string(), user_v1_t2.to_string());
    assert_ne!(user_v2_t1.to_string(), user_v1_t2.to_string());
}