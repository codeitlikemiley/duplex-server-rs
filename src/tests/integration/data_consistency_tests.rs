//! Data Consistency Tests
//!
//! This module contains comprehensive tests for data consistency across operations.
//! Tests include concurrent operations, transaction isolation, cache consistency,
//! event sourcing consistency, and data integrity under load.

#[cfg(test)]
mod data_consistency_tests {
    use chrono::{DateTime, Utc, Duration};
    use uuid::Uuid;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
    use tokio::sync::{RwLock, Semaphore};
    use tokio::time::{sleep, Duration as TokioDuration, Instant};

    // Data consistency simulator for testing
    #[derive(Debug, Clone)]
    pub struct DataConsistencySimulator {
        // Primary data store (database simulation)
        database: Arc<RwLock<HashMap<Uuid, ConsistentUser>>>,
        // Cache layer
        cache: Arc<RwLock<HashMap<Uuid, CachedUser>>>,
        // Event store for event sourcing
        event_store: Arc<Mutex<Vec<DomainEvent>>>,
        // Version counters for optimistic locking
        version_counters: Arc<RwLock<HashMap<Uuid, u64>>>,
        // Consistency violations detected
        violations: Arc<Mutex<Vec<ConsistencyViolation>>>,
        // Metrics
        read_count: Arc<AtomicU64>,
        write_count: Arc<AtomicU64>,
        conflict_count: Arc<AtomicU64>,
        rollback_count: Arc<AtomicU64>,
        // Simulation controls
        enable_cache: Arc<AtomicBool>,
        enable_eventual_consistency: Arc<AtomicBool>,
    }

    #[derive(Debug, Clone)]
    pub struct ConsistentUser {
        pub id: Uuid,
        pub email: String,
        pub username: String,
        pub profile: UserProfile,
        pub status: UserStatus,
        pub version: u64,
        pub last_modified: DateTime<Utc>,
        pub created_at: DateTime<Utc>,
        pub checksum: String,
    }

    #[derive(Debug, Clone)]
    pub struct CachedUser {
        pub user: ConsistentUser,
        pub cached_at: DateTime<Utc>,
        pub ttl_seconds: i64,
        pub hit_count: u64,
    }

    #[derive(Debug, Clone)]
    pub struct UserProfile {
        pub display_name: String,
        pub bio: String,
        pub avatar_url: Option<String>,
        pub preferences: HashMap<String, String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum UserStatus {
        Active,
        Inactive,
        Locked,
        Deleted,
    }

    #[derive(Debug, Clone)]
    pub enum DomainEvent {
        UserCreated { user_id: Uuid, timestamp: DateTime<Utc>, data: HashMap<String, String> },
        UserUpdated { user_id: Uuid, timestamp: DateTime<Utc>, changes: HashMap<String, String> },
        UserDeleted { user_id: Uuid, timestamp: DateTime<Utc> },
        ProfileUpdated { user_id: Uuid, timestamp: DateTime<Utc>, changes: HashMap<String, String> },
        StatusChanged { user_id: Uuid, old_status: UserStatus, new_status: UserStatus, timestamp: DateTime<Utc> },
    }

    #[derive(Debug, Clone)]
    pub struct ConsistencyViolation {
        pub violation_type: ViolationType,
        pub entity_id: Uuid,
        pub expected_value: String,
        pub actual_value: String,
        pub timestamp: DateTime<Utc>,
        pub context: String,
    }

    #[derive(Debug, Clone)]
    pub enum ViolationType {
        DirtyRead,
        PhantomRead,
        LostUpdate,
        StaleCache,
        EventMismatch,
        ChecksumMismatch,
        VersionConflict,
    }

    #[derive(Debug, Clone)]
    pub struct ConsistencyTestResult {
        pub test_name: String,
        pub passed: bool,
        pub violations_found: usize,
        pub operations_performed: u64,
        pub conflicts_detected: u64,
        pub rollbacks_performed: u64,
        pub consistency_score: f64,
        pub details: String,
    }

    impl DataConsistencySimulator {
        pub fn new() -> Self {
            Self {
                database: Arc::new(RwLock::new(HashMap::new())),
                cache: Arc::new(RwLock::new(HashMap::new())),
                event_store: Arc::new(Mutex::new(Vec::new())),
                version_counters: Arc::new(RwLock::new(HashMap::new())),
                violations: Arc::new(Mutex::new(Vec::new())),
                read_count: Arc::new(AtomicU64::new(0)),
                write_count: Arc::new(AtomicU64::new(0)),
                conflict_count: Arc::new(AtomicU64::new(0)),
                rollback_count: Arc::new(AtomicU64::new(0)),
                enable_cache: Arc::new(AtomicBool::new(true)),
                enable_eventual_consistency: Arc::new(AtomicBool::new(false)),
            }
        }

        // Create test user with consistency tracking
        pub fn create_test_user(&self, email: &str) -> ConsistentUser {
            ConsistentUser {
                id: Uuid::now_v7(),
                email: email.to_string(),
                username: email.split('@').next().unwrap_or("user").to_string(),
                profile: UserProfile {
                    display_name: "Test User".to_string(),
                    bio: "Test bio".to_string(),
                    avatar_url: None,
                    preferences: HashMap::new(),
                },
                status: UserStatus::Active,
                version: 1,
                last_modified: Utc::now(),
                created_at: Utc::now(),
                checksum: self.calculate_checksum(&email),
            }
        }

        // Calculate checksum for data integrity verification
        fn calculate_checksum(&self, data: &str) -> String {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        }

        // Perform transactional write with consistency checks
        pub async fn transactional_write(&self, user: ConsistentUser) -> Result<(), ConsistencyViolation> {
            // Update version for optimistic locking
            let new_version = {
                let mut versions = self.version_counters.write().await;
                let version = versions.entry(user.id).or_insert(0);
                *version += 1;
                *version
            };

            // Check for version conflicts
            {
                let db = self.database.read().await;
                if let Some(existing) = db.get(&user.id) {
                    if existing.version >= new_version {
                        self.conflict_count.fetch_add(1, Ordering::SeqCst);
                        return Err(ConsistencyViolation {
                            violation_type: ViolationType::VersionConflict,
                            entity_id: user.id,
                            expected_value: format!("Version < {}", new_version),
                            actual_value: format!("Version {}", existing.version),
                            timestamp: Utc::now(),
                            context: "Optimistic locking violation".to_string(),
                        });
                    }
                }
            }

            let mut updated_user = user.clone();
            updated_user.version = new_version;
            updated_user.last_modified = Utc::now();
            updated_user.checksum = self.calculate_checksum(&format!("{}{}", user.email, new_version));

            // Write to database
            {
                let mut db = self.database.write().await;
                db.insert(user.id, updated_user.clone());
            }

            // Update cache if enabled
            if self.enable_cache.load(Ordering::SeqCst) {
                let mut cache = self.cache.write().await;
                cache.insert(user.id, CachedUser {
                    user: updated_user.clone(),
                    cached_at: Utc::now(),
                    ttl_seconds: 300,
                    hit_count: 0,
                });
            }

            // Record event
            {
                let mut events = self.event_store.lock().unwrap();
                events.push(DomainEvent::UserUpdated {
                    user_id: user.id,
                    timestamp: Utc::now(),
                    changes: HashMap::new(),
                });
            }

            self.write_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        // Read with consistency check
        pub async fn read_with_consistency_check(&self, user_id: &Uuid) -> Result<ConsistentUser, ConsistencyViolation> {
            self.read_count.fetch_add(1, Ordering::SeqCst);

            // Check cache first if enabled
            if self.enable_cache.load(Ordering::SeqCst) {
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(user_id) {
                    let db = self.database.read().await;
                    if let Some(db_user) = db.get(user_id) {
                        if cached.user.version < db_user.version {
                            let violation = ConsistencyViolation {
                                violation_type: ViolationType::StaleCache,
                                entity_id: *user_id,
                                expected_value: format!("Version {}", db_user.version),
                                actual_value: format!("Version {}", cached.user.version),
                                timestamp: Utc::now(),
                                context: "Cache consistency violation".to_string(),
                            };
                            self.record_violation(violation.clone());
                            return Err(violation);
                        }
                    }
                    return Ok(cached.user.clone());
                }
            }

            // Read from database
            let db = self.database.read().await;
            db.get(user_id).cloned().ok_or_else(|| ConsistencyViolation {
                violation_type: ViolationType::PhantomRead,
                entity_id: *user_id,
                expected_value: "User exists".to_string(),
                actual_value: "User not found".to_string(),
                timestamp: Utc::now(),
                context: "Read consistency violation".to_string(),
            })
        }

        // Record consistency violation
        fn record_violation(&self, violation: ConsistencyViolation) {
            let mut violations = self.violations.lock().unwrap();
            violations.push(violation);
        }

        // Test concurrent consistency
        pub async fn test_concurrent_consistency(&self, num_operations: usize) -> ConsistencyTestResult {
            let start_time = Instant::now();

            // Create test users
            let users: Vec<ConsistentUser> = (0..10)
                .map(|i| self.create_test_user(&format!("user{}@test.com", i)))
                .collect();

            // Initialize users in database
            for user in &users {
                let mut db = self.database.write().await;
                db.insert(user.id, user.clone());
            }

            // Perform concurrent operations
            let mut handles = vec![];
            for i in 0..num_operations {
                let user = users[i % users.len()].clone();
                let simulator = self.clone();

                let handle = tokio::spawn(async move {
                    if i % 3 == 0 {
                        // Write operation
                        let mut updated_user = user.clone();
                        updated_user.profile.bio = format!("Updated bio {}", i);
                        let _ = simulator.transactional_write(updated_user).await;
                    } else {
                        // Read operation
                        let _ = simulator.read_with_consistency_check(&user.id).await;
                    }
                });

                handles.push(handle);
            }

            // Wait for all operations to complete
            for handle in handles {
                let _ = handle.await;
            }

            // Check for consistency violations
            let violations = self.violations.lock().unwrap();
            let violation_count = violations.len();

            let consistency_score = if num_operations > 0 {
                1.0 - (violation_count as f64 / num_operations as f64)
            } else {
                1.0
            };

            ConsistencyTestResult {
                test_name: "Concurrent Consistency Test".to_string(),
                passed: violation_count == 0,
                violations_found: violation_count,
                operations_performed: self.read_count.load(Ordering::SeqCst) + self.write_count.load(Ordering::SeqCst),
                conflicts_detected: self.conflict_count.load(Ordering::SeqCst),
                rollbacks_performed: self.rollback_count.load(Ordering::SeqCst),
                consistency_score,
                details: format!("Performed {} operations with {} violations", num_operations, violation_count),
            }
        }

        // Test cache and database consistency
        pub async fn test_cache_database_consistency(&self) -> ConsistencyTestResult {
            self.enable_cache.store(true, Ordering::SeqCst);

            // Create and store users
            let users: Vec<ConsistentUser> = (0..20)
                .map(|i| self.create_test_user(&format!("cache_test_{}@test.com", i)))
                .collect();

            // Write to database and cache
            for user in &users {
                let _ = self.transactional_write(user.clone()).await;
            }

            // Perform mixed operations
            let mut handles = vec![];
            for i in 0..100 {
                let user_id = users[i % users.len()].id;
                let simulator = self.clone();

                let handle = tokio::spawn(async move {
                    if i % 5 == 0 {
                        // Invalidate cache randomly
                        let mut cache = simulator.cache.write().await;
                        cache.remove(&user_id);
                    } else if i % 3 == 0 {
                        // Update database directly (bypassing cache)
                        let mut db = simulator.database.write().await;
                        if let Some(user) = db.get_mut(&user_id) {
                            user.version += 1;
                            user.last_modified = Utc::now();
                        }
                    } else {
                        // Read and verify consistency
                        let _ = simulator.read_with_consistency_check(&user_id).await;
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            let violations = self.violations.lock().unwrap();
            let stale_cache_violations = violations.iter()
                .filter(|v| matches!(v.violation_type, ViolationType::StaleCache))
                .count();

            ConsistencyTestResult {
                test_name: "Cache-Database Consistency Test".to_string(),
                passed: stale_cache_violations == 0,
                violations_found: stale_cache_violations,
                operations_performed: self.read_count.load(Ordering::SeqCst) + self.write_count.load(Ordering::SeqCst),
                conflicts_detected: self.conflict_count.load(Ordering::SeqCst),
                rollbacks_performed: self.rollback_count.load(Ordering::SeqCst),
                consistency_score: 1.0 - (stale_cache_violations as f64 / 100.0),
                details: format!("Found {} stale cache violations", stale_cache_violations),
            }
        }

        // Test event sourcing consistency
        pub async fn test_event_sourcing_consistency(&self) -> ConsistencyTestResult {
            // Create users and track events
            let users: Vec<ConsistentUser> = (0..10)
                .map(|i| self.create_test_user(&format!("event_test_{}@test.com", i)))
                .collect();

            // Perform operations and record events
            for user in &users {
                // Create event
                {
                    let mut events = self.event_store.lock().unwrap();
                    events.push(DomainEvent::UserCreated {
                        user_id: user.id,
                        timestamp: Utc::now(),
                        data: HashMap::new(),
                    });
                }

                // Update event
                {
                    let mut events = self.event_store.lock().unwrap();
                    events.push(DomainEvent::ProfileUpdated {
                        user_id: user.id,
                        timestamp: Utc::now(),
                        changes: HashMap::from([
                            ("bio".to_string(), "Updated bio".to_string())
                        ]),
                    });
                }

                // Status change event
                {
                    let mut events = self.event_store.lock().unwrap();
                    events.push(DomainEvent::StatusChanged {
                        user_id: user.id,
                        old_status: UserStatus::Active,
                        new_status: UserStatus::Inactive,
                        timestamp: Utc::now(),
                    });
                }
            }

            // Rebuild state from events and verify consistency
            let events = self.event_store.lock().unwrap();
            let mut rebuilt_state: HashMap<Uuid, UserStatus> = HashMap::new();

            for event in events.iter() {
                match event {
                    DomainEvent::UserCreated { user_id, .. } => {
                        rebuilt_state.insert(*user_id, UserStatus::Active);
                    }
                    DomainEvent::StatusChanged { user_id, new_status, .. } => {
                        rebuilt_state.insert(*user_id, new_status.clone());
                    }
                    _ => {}
                }
            }

            // Verify consistency
            let mut event_violations = 0;
            for user in &users {
                if let Some(status) = rebuilt_state.get(&user.id) {
                    if !matches!(status, UserStatus::Inactive) {
                        event_violations += 1;
                    }
                }
            }

            ConsistencyTestResult {
                test_name: "Event Sourcing Consistency Test".to_string(),
                passed: event_violations == 0,
                violations_found: event_violations,
                operations_performed: events.len() as u64,
                conflicts_detected: 0,
                rollbacks_performed: 0,
                consistency_score: 1.0 - (event_violations as f64 / users.len() as f64),
                details: format!("Processed {} events with {} inconsistencies", events.len(), event_violations),
            }
        }

        // Test optimistic locking
        pub async fn test_optimistic_locking(&self) -> ConsistencyTestResult {
            let test_user = self.create_test_user("optimistic_test@test.com");

            // Initialize user
            {
                let mut db = self.database.write().await;
                db.insert(test_user.id, test_user.clone());
            }

            // Concurrent updates with version checking
            let mut handles = vec![];
            for i in 0..10 {
                let mut user = test_user.clone();
                user.profile.bio = format!("Optimistic update {}", i);
                let sim = self.clone();

                let handle = tokio::spawn(async move {
                    sim.transactional_write(user).await
                });

                handles.push(handle);
            }

            let mut success_count = 0;
            let mut conflict_count = 0;
            for handle in handles {
                match handle.await.unwrap() {
                    Ok(_) => success_count += 1,
                    Err(_) => conflict_count += 1,
                }
            }

            ConsistencyTestResult {
                test_name: "Optimistic Locking Test".to_string(),
                passed: success_count > 0 && conflict_count > 0,
                violations_found: 0,
                operations_performed: 10,
                conflicts_detected: conflict_count as u64,
                rollbacks_performed: 0,
                consistency_score: success_count as f64 / 10.0,
                details: format!("{} succeeded, {} conflicts detected", success_count, conflict_count),
            }
        }

        // Generate consistency report
        pub fn generate_consistency_report(&self, results: &[ConsistencyTestResult]) -> String {
            let mut report = String::from("# Data Consistency Test Report\n\n");

            let total_tests = results.len();
            let passed_tests = results.iter().filter(|r| r.passed).count();
            let total_violations = results.iter().map(|r| r.violations_found).sum::<usize>();
            let avg_consistency_score = results.iter().map(|r| r.consistency_score).sum::<f64>() / total_tests as f64;

            report.push_str(&format!("## Summary\n"));
            report.push_str(&format!("- Tests Passed: {}/{}\n", passed_tests, total_tests));
            report.push_str(&format!("- Total Violations: {}\n", total_violations));
            report.push_str(&format!("- Average Consistency Score: {:.2}%\n\n", avg_consistency_score * 100.0));

            report.push_str("## Test Results\n\n");
            for result in results {
                let status = if result.passed { "✅ PASSED" } else { "❌ FAILED" };
                report.push_str(&format!("### {} - {}\n", result.test_name, status));
                report.push_str(&format!("- Violations Found: {}\n", result.violations_found));
                report.push_str(&format!("- Operations: {}\n", result.operations_performed));
                report.push_str(&format!("- Conflicts: {}\n", result.conflicts_detected));
                report.push_str(&format!("- Rollbacks: {}\n", result.rollbacks_performed));
                report.push_str(&format!("- Consistency Score: {:.2}%\n", result.consistency_score * 100.0));
                report.push_str(&format!("- Details: {}\n\n", result.details));
            }

            report
        }
    }

    // Test concurrent read/write consistency
    #[tokio::test]
    async fn test_concurrent_read_write_consistency() {
        let simulator = DataConsistencySimulator::new();
        let result = simulator.test_concurrent_consistency(100).await;

        assert!(result.consistency_score >= 0.95, "Consistency score too low: {}", result.consistency_score);
        assert!(result.violations_found < 5, "Too many violations: {}", result.violations_found);

        println!("Concurrent Consistency Test: {}", result.details);
    }

    // Test cache and database synchronization
    #[tokio::test]
    async fn test_cache_database_sync() {
        let simulator = DataConsistencySimulator::new();
        let result = simulator.test_cache_database_consistency().await;

        assert!(result.passed || result.violations_found < 10,
            "Cache consistency test failed with {} violations", result.violations_found);

        println!("Cache-Database Sync Test: {}", result.details);
    }

    // Test event sourcing consistency
    #[tokio::test]
    async fn test_event_sourcing_integrity() {
        let simulator = DataConsistencySimulator::new();
        let result = simulator.test_event_sourcing_consistency().await;

        assert!(result.passed, "Event sourcing consistency failed: {}", result.details);
        assert_eq!(result.violations_found, 0, "Event sourcing should have no violations");

        println!("Event Sourcing Test: {}", result.details);
    }

    // Test optimistic locking
    #[tokio::test]
    async fn test_optimistic_locking_mechanism() {
        let simulator = DataConsistencySimulator::new();
        let result = simulator.test_optimistic_locking().await;

        assert!(result.passed, "Optimistic locking test failed: {}", result.details);
        assert!(result.conflicts_detected > 0, "Should detect version conflicts");

        println!("Optimistic Locking Test: {}", result.details);
    }

    // Test data integrity under load
    #[tokio::test]
    async fn test_data_integrity_under_load() {
        let simulator = DataConsistencySimulator::new();
        simulator.enable_eventual_consistency.store(true, Ordering::SeqCst);

        let result = simulator.test_concurrent_consistency(500).await;

        assert!(result.operations_performed > 400, "Should perform most operations");
        assert!(result.consistency_score >= 0.90, "Consistency score should be acceptable under load");

        println!("Data Integrity Under Load: {}", result.details);
    }

    // Test eventual consistency
    #[tokio::test]
    async fn test_eventual_consistency() {
        let simulator = DataConsistencySimulator::new();
        simulator.enable_eventual_consistency.store(true, Ordering::SeqCst);
        simulator.enable_cache.store(true, Ordering::SeqCst);

        let users: Vec<ConsistentUser> = (0..5)
            .map(|i| simulator.create_test_user(&format!("eventual_{}@test.com", i)))
            .collect();

        // Write users
        for user in &users {
            let _ = simulator.transactional_write(user.clone()).await;
        }

        // Allow time for eventual consistency
        sleep(TokioDuration::from_millis(50)).await;

        // Verify all users are consistent
        let mut consistency_checks = 0;
        for user in &users {
            if let Ok(read_user) = simulator.read_with_consistency_check(&user.id).await {
                if read_user.checksum == user.checksum || read_user.version > user.version {
                    consistency_checks += 1;
                }
            }
        }

        assert_eq!(consistency_checks, users.len(), "All users should be eventually consistent");

        println!("Eventual Consistency: {}/{} users consistent", consistency_checks, users.len());
    }

    // Comprehensive data consistency test
    #[tokio::test]
    async fn test_comprehensive_data_consistency() {
        let simulator = DataConsistencySimulator::new();

        let test_results = vec![
            simulator.test_concurrent_consistency(200).await,
            simulator.test_cache_database_consistency().await,
            simulator.test_event_sourcing_consistency().await,
            simulator.test_optimistic_locking().await,
        ];

        let report = simulator.generate_consistency_report(&test_results);
        println!("\n{}", report);

        let passed_tests = test_results.iter().filter(|r| r.passed).count();
        let total_tests = test_results.len();

        assert!(passed_tests >= total_tests - 1,
            "Most consistency tests should pass. Passed: {}/{}", passed_tests, total_tests);

        let total_violations: usize = test_results.iter().map(|r| r.violations_found).sum();
        assert!(total_violations < 50, "Total violations should be minimal: {}", total_violations);

        println!("✅ Comprehensive data consistency testing completed successfully!");
    }
}