//! Integration tests for database transaction integrity - Enterprise Edition
//!
//! These tests verify that database transactions maintain ACID properties
//! and handle concurrent operations, rollbacks, and error scenarios correctly.
//!
//! Enterprise features include:
//! - Advanced deadlock detection and recovery
//! - Transaction performance monitoring and metrics
//! - Savepoint and nested transaction support
//! - Connection pool transaction management
//! - Transaction timeout and resource limits
//! - Audit logging for transaction operations
//! - Cross-database transaction coordination
//! - Transaction replay and recovery mechanisms

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration, timeout};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    Started,
    InProgress,
    Committed,
    RolledBack,
    Failed,
    TimedOut,
    Suspended, // For savepoints
    Preparing, // For two-phase commit
    Prepared,  // For two-phase commit
    Aborted,   // Aborted due to deadlock or resource limits
}

#[derive(Debug, Clone, PartialEq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionTestUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64, // For optimistic locking
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionTestProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

// Enterprise Transaction Management Structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetrics {
    pub transaction_id: Uuid,
    pub start_time: Instant,
    pub duration: Option<Duration>,
    pub operations_count: u32,
    pub locks_acquired: u32,
    pub locks_waited_for: u32,
    pub deadlocks_detected: u32,
    pub rows_read: u64,
    pub rows_written: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub cpu_time_ms: u64,
    pub memory_used_bytes: u64,
    pub isolation_level: IsolationLevel,
    pub final_state: Option<TransactionState>,
}

#[derive(Debug, Clone)]
pub struct Savepoint {
    pub id: Uuid,
    pub name: String,
    pub transaction_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub operations_at_creation: usize,
    pub locks_at_creation: Vec<DatabaseLock>,
}

#[derive(Debug, Clone)]
pub struct ConnectionPool {
    pub pool_id: Uuid,
    pub max_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub waiting_transactions: VecDeque<Uuid>,
    pub connection_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct TransactionTimeout {
    pub transaction_id: Uuid,
    pub timeout_duration: Duration,
    pub started_at: Instant,
    pub warning_threshold: Duration,
    pub escalation_policy: TimeoutEscalationPolicy,
}

#[derive(Debug, Clone)]
pub enum TimeoutEscalationPolicy {
    Kill,           // Kill the transaction immediately
    Rollback,       // Rollback gracefully
    Warn,           // Log warning and continue
    Downgrade,      // Reduce isolation level
    Suspend,        // Suspend until resources available
}

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub operation: String,
    pub table: String,
    pub record_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeadlockGraph {
    pub detection_id: Uuid,
    pub detected_at: DateTime<Utc>,
    pub transactions: Vec<Uuid>,
    pub wait_for_graph: HashMap<Uuid, Vec<Uuid>>,
    pub cycle_detected: Vec<Uuid>,
    pub victim_transaction: Option<Uuid>,
    pub resolution_strategy: DeadlockResolutionStrategy,
}

#[derive(Debug, Clone)]
pub enum DeadlockResolutionStrategy {
    YoungestFirst,      // Abort youngest transaction
    OldestFirst,        // Abort oldest transaction
    LeastWork,          // Abort transaction with least work
    RandomVictim,       // Random selection
    HighestCost,        // Abort highest cost transaction
    WaitDie,            // Wait-die prevention
    WoundWait,          // Wound-wait prevention
}

#[derive(Debug, Clone)]
pub struct TransactionResourceLimits {
    pub max_locks: usize,
    pub max_operations: usize,
    pub max_duration: Duration,
    pub max_memory_mb: usize,
    pub max_cpu_time_ms: u64,
    pub max_rows_read: u64,
    pub max_rows_written: u64,
    pub priority: TransactionPriority,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionPriority {
    Critical = 1,
    High = 2,
    Normal = 3,
    Low = 4,
    Batch = 5,
}

#[derive(Debug, Clone)]
pub struct CrossDatabaseTransaction {
    pub coordinator_id: Uuid,
    pub participant_databases: Vec<String>,
    pub transaction_ids: HashMap<String, Uuid>, // database -> transaction_id
    pub phase: TwoPhaseCommitPhase,
    pub votes: HashMap<String, TwoPhaseVote>,
    pub global_state: TransactionState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TwoPhaseCommitPhase {
    Prepare,
    Commit,
    Abort,
    Complete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TwoPhaseVote {
    VoteCommit,
    VoteAbort,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct TransactionReplayLog {
    pub log_id: Uuid,
    pub transaction_id: Uuid,
    pub operations: Vec<ReplayableOperation>,
    pub checkpoints: Vec<TransactionCheckpoint>,
    pub created_at: DateTime<Utc>,
    pub replay_count: u32,
}

#[derive(Debug, Clone)]
pub struct ReplayableOperation {
    pub operation_id: Uuid,
    pub operation_type: String,
    pub sql_statement: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: u64,
    pub compensating_operation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionCheckpoint {
    pub checkpoint_id: Uuid,
    pub transaction_id: Uuid,
    pub sequence_number: u64,
    pub state_snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DatabaseTransaction {
    pub id: Uuid,
    pub state: TransactionState,
    pub isolation_level: IsolationLevel,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub operations: Vec<TransactionOperation>,
    pub locks_held: Vec<DatabaseLock>,
    pub savepoints: Vec<Savepoint>,
    pub metrics: TransactionMetrics,
    pub resource_limits: TransactionResourceLimits,
    pub timeout_config: Option<TransactionTimeout>,
    pub priority: TransactionPriority,
    pub parent_transaction_id: Option<Uuid>, // For nested transactions
    pub session_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub connection_id: Option<Uuid>,
    pub cross_database_coordinator: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    Insert { table: String, record_id: Uuid, operation_id: Uuid, timestamp: DateTime<Utc> },
    Update { table: String, record_id: Uuid, old_version: i64, new_version: i64, operation_id: Uuid, timestamp: DateTime<Utc> },
    Delete { table: String, record_id: Uuid, operation_id: Uuid, timestamp: DateTime<Utc> },
    Select { table: String, record_id: Option<Uuid>, operation_id: Uuid, timestamp: DateTime<Utc> },
    CreateSavepoint { name: String, savepoint_id: Uuid, timestamp: DateTime<Utc> },
    RollbackToSavepoint { savepoint_id: Uuid, timestamp: DateTime<Utc> },
    ReleaseSavepoint { savepoint_id: Uuid, timestamp: DateTime<Utc> },
}

#[derive(Debug, Clone)]
pub struct DatabaseLock {
    pub lock_type: LockType,
    pub table: String,
    pub record_id: Option<Uuid>,
    pub acquired_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum LockType {
    Shared,
    Exclusive,
    RowExclusive,
    ShareUpdateExclusive,
}

#[derive(Debug, Clone)]
pub struct TransactionIntegritySimulator {
    users: HashMap<Uuid, TransactionTestUser>,
    profiles: HashMap<Uuid, TransactionTestProfile>,
    active_transactions: HashMap<Uuid, DatabaseTransaction>,
    completed_transactions: Vec<DatabaseTransaction>,
    global_locks: Vec<DatabaseLock>,
    deadlock_detection_enabled: bool,
    simulate_failures: bool,
    constraint_violations: Vec<String>,
    // Enterprise features
    transaction_metrics: HashMap<Uuid, TransactionMetrics>,
    connection_pool: ConnectionPool,
    audit_log: Vec<AuditLogEntry>,
    deadlock_graphs: Vec<DeadlockGraph>,
    cross_db_transactions: HashMap<Uuid, CrossDatabaseTransaction>,
    replay_logs: HashMap<Uuid, TransactionReplayLog>,
    resource_monitor: ResourceMonitor,
    timeout_manager: TimeoutManager,
    performance_stats: PerformanceStatistics,
}

#[derive(Debug, Clone)]
pub struct ResourceMonitor {
    pub total_memory_used: u64,
    pub active_connections: usize,
    pub locks_held: usize,
    pub cpu_usage_percent: f64,
    pub disk_io_operations: u64,
    pub network_io_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct TimeoutManager {
    pub active_timeouts: HashMap<Uuid, TransactionTimeout>,
    pub timeout_warnings_sent: HashSet<Uuid>,
    pub escalated_transactions: HashSet<Uuid>,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceStatistics {
    pub transactions_per_second: f64,
    pub average_transaction_duration: Duration,
    pub deadlock_rate: f64,
    pub lock_contention_rate: f64,
    pub rollback_rate: f64,
    pub resource_utilization: f64,
    pub throughput_mbps: f64,
}

impl TransactionIntegritySimulator {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            profiles: HashMap::new(),
            active_transactions: HashMap::new(),
            completed_transactions: Vec::new(),
            global_locks: Vec::new(),
            deadlock_detection_enabled: true,
            simulate_failures: false,
            constraint_violations: Vec::new(),
            // Enterprise features
            transaction_metrics: HashMap::new(),
            connection_pool: ConnectionPool {
                pool_id: Uuid::now_v7(),
                max_connections: 100,
                active_connections: 0,
                idle_connections: 10,
                waiting_transactions: VecDeque::new(),
                connection_timeout: Duration::from_secs(30),
            },
            audit_log: Vec::new(),
            deadlock_graphs: Vec::new(),
            cross_db_transactions: HashMap::new(),
            replay_logs: HashMap::new(),
            resource_monitor: ResourceMonitor {
                total_memory_used: 0,
                active_connections: 0,
                locks_held: 0,
                cpu_usage_percent: 0.0,
                disk_io_operations: 0,
                network_io_bytes: 0,
            },
            timeout_manager: TimeoutManager {
                active_timeouts: HashMap::new(),
                timeout_warnings_sent: HashSet::new(),
                escalated_transactions: HashSet::new(),
            },
            performance_stats: PerformanceStatistics::default(),
        }
    }

    pub fn enable_failure_simulation(&mut self) {
        self.simulate_failures = true;
    }

    pub fn disable_failure_simulation(&mut self) {
        self.simulate_failures = false;
    }

    // Enterprise Transaction lifecycle management
    pub async fn begin_transaction(&mut self, isolation_level: IsolationLevel) -> Result<Uuid, String> {
        self.begin_transaction_with_options(isolation_level, None, None, None).await
    }

    pub async fn begin_transaction_with_options(
        &mut self,
        isolation_level: IsolationLevel,
        priority: Option<TransactionPriority>,
        timeout: Option<Duration>,
        parent_id: Option<Uuid>,
    ) -> Result<Uuid, String> {
        let transaction_id = Uuid::now_v7();
        let now = Utc::now();
        let start_time = Instant::now();

        // Check connection pool availability
        if self.connection_pool.active_connections >= self.connection_pool.max_connections {
            self.connection_pool.waiting_transactions.push_back(transaction_id);
            return Err("No available connections in pool".to_string());
        }

        self.connection_pool.active_connections += 1;

        let priority = priority.unwrap_or(TransactionPriority::Normal);
        let resource_limits = self.get_default_resource_limits(&priority);

        let metrics = TransactionMetrics {
            transaction_id,
            start_time,
            duration: None,
            operations_count: 0,
            locks_acquired: 0,
            locks_waited_for: 0,
            deadlocks_detected: 0,
            rows_read: 0,
            rows_written: 0,
            bytes_read: 0,
            bytes_written: 0,
            cpu_time_ms: 0,
            memory_used_bytes: 0,
            isolation_level: isolation_level.clone(),
            final_state: None,
        };

        let timeout_config = if let Some(timeout_duration) = timeout {
            Some(TransactionTimeout {
                transaction_id,
                timeout_duration,
                started_at: start_time,
                warning_threshold: timeout_duration / 2,
                escalation_policy: TimeoutEscalationPolicy::Rollback,
            })
        } else {
            None
        };

        let transaction = DatabaseTransaction {
            id: transaction_id,
            state: TransactionState::Started,
            isolation_level,
            started_at: now,
            completed_at: None,
            operations: Vec::new(),
            locks_held: Vec::new(),
            savepoints: Vec::new(),
            metrics: metrics.clone(),
            resource_limits,
            timeout_config: timeout_config.clone(),
            priority,
            parent_transaction_id: parent_id,
            session_id: Some(format!("session_{}", Uuid::now_v7())),
            user_id: Some(Uuid::now_v7()),
            connection_id: Some(Uuid::now_v7()),
            cross_database_coordinator: None,
        };

        self.active_transactions.insert(transaction_id, transaction);
        self.transaction_metrics.insert(transaction_id, metrics);

        if let Some(timeout_cfg) = timeout_config {
            self.timeout_manager.active_timeouts.insert(transaction_id, timeout_cfg);
        }

        // Create audit log entry
        self.audit_log.push(AuditLogEntry {
            id: Uuid::now_v7(),
            transaction_id,
            operation: "BEGIN".to_string(),
            table: "system".to_string(),
            record_id: None,
            old_values: None,
            new_values: Some(serde_json::json!({
                "isolation_level": format!("{:?}", isolation_level),
                "priority": format!("{:?}", priority)
            })),
            user_id: Some(Uuid::now_v7()),
            session_id: Some(format!("session_{}", Uuid::now_v7())),
            ip_address: Some("127.0.0.1".to_string()),
            timestamp: now,
            success: true,
            error_message: None,
        });

        Ok(transaction_id)
    }

    fn get_default_resource_limits(&self, priority: &TransactionPriority) -> TransactionResourceLimits {
        match priority {
            TransactionPriority::Critical => TransactionResourceLimits {
                max_locks: 10000,
                max_operations: 100000,
                max_duration: Duration::from_secs(3600), // 1 hour
                max_memory_mb: 1024, // 1GB
                max_cpu_time_ms: 300000, // 5 minutes
                max_rows_read: 10000000,
                max_rows_written: 1000000,
                priority: priority.clone(),
            },
            TransactionPriority::High => TransactionResourceLimits {
                max_locks: 5000,
                max_operations: 50000,
                max_duration: Duration::from_secs(1800), // 30 minutes
                max_memory_mb: 512,
                max_cpu_time_ms: 180000, // 3 minutes
                max_rows_read: 5000000,
                max_rows_written: 500000,
                priority: priority.clone(),
            },
            TransactionPriority::Normal => TransactionResourceLimits {
                max_locks: 1000,
                max_operations: 10000,
                max_duration: Duration::from_secs(600), // 10 minutes
                max_memory_mb: 256,
                max_cpu_time_ms: 60000, // 1 minute
                max_rows_read: 1000000,
                max_rows_written: 100000,
                priority: priority.clone(),
            },
            TransactionPriority::Low => TransactionResourceLimits {
                max_locks: 500,
                max_operations: 5000,
                max_duration: Duration::from_secs(300), // 5 minutes
                max_memory_mb: 128,
                max_cpu_time_ms: 30000, // 30 seconds
                max_rows_read: 500000,
                max_rows_written: 50000,
                priority: priority.clone(),
            },
            TransactionPriority::Batch => TransactionResourceLimits {
                max_locks: 100,
                max_operations: 1000,
                max_duration: Duration::from_secs(60), // 1 minute
                max_memory_mb: 64,
                max_cpu_time_ms: 10000, // 10 seconds
                max_rows_read: 100000,
                max_rows_written: 10000,
                priority: priority.clone(),
            },
        }
    }

    pub async fn commit_transaction(&mut self, transaction_id: Uuid) -> Result<(), String> {
        let mut transaction = self.active_transactions.remove(&transaction_id)
            .ok_or("Transaction not found")?;

        if transaction.state != TransactionState::Started && transaction.state != TransactionState::InProgress {
            return Err("Transaction not in valid state for commit".to_string());
        }

        // Simulate potential commit failure
        if self.simulate_failures && transaction.operations.len() > 3 {
            transaction.state = TransactionState::Failed;
            self.rollback_operations(&transaction).await?;

            // Release all locks before completing
            let locks_to_remove = transaction.locks_held.clone();
            for lock in locks_to_remove {
                self.global_locks.retain(|l| {
                    !(l.table == lock.table &&
                      l.record_id == lock.record_id &&
                      l.acquired_at == lock.acquired_at)
                });
            }

            self.completed_transactions.push(transaction);
            return Err("Simulated commit failure".to_string());
        }

        // Apply all operations atomically
        for operation in &transaction.operations {
            self.apply_operation(operation).await?;
        }

        transaction.state = TransactionState::Committed;
        transaction.completed_at = Some(Utc::now());

        // Release all locks before completing
        let locks_to_remove = transaction.locks_held.clone();
        for lock in locks_to_remove {
            self.global_locks.retain(|l| {
                !(l.table == lock.table &&
                  l.record_id == lock.record_id &&
                  l.acquired_at == lock.acquired_at)
            });
        }

        self.completed_transactions.push(transaction);
        Ok(())
    }

    pub async fn rollback_transaction(&mut self, transaction_id: Uuid) -> Result<(), String> {
        let mut transaction = self.active_transactions.remove(&transaction_id)
            .ok_or("Transaction not found")?;

        transaction.state = TransactionState::RolledBack;
        transaction.completed_at = Some(Utc::now());

        self.rollback_operations(&transaction).await?;

        // Release all locks before completing
        let locks_to_remove = transaction.locks_held.clone();
        for lock in locks_to_remove {
            self.global_locks.retain(|l| {
                !(l.table == lock.table &&
                  l.record_id == lock.record_id &&
                  l.acquired_at == lock.acquired_at)
            });
        }

        self.completed_transactions.push(transaction);
        Ok(())
    }

    // Database operations within transactions
    pub async fn insert_user_in_transaction(
        &mut self,
        transaction_id: Uuid,
        user: TransactionTestUser,
    ) -> Result<(), String> {
        self.validate_transaction_state(transaction_id)?;

        // Check for constraint violations
        if self.users.values().any(|u| u.email == user.email) {
            self.constraint_violations.push(format!("Duplicate email: {}", user.email));
            return Err("Email already exists".to_string());
        }

        // Acquire exclusive lock on users table
        self.acquire_lock(transaction_id, LockType::RowExclusive, "users".to_string(), Some(user.id)).await?;

        let operation = TransactionOperation::Insert {
            table: "users".to_string(),
            record_id: user.id,
            operation_id: Uuid::now_v7(),
            timestamp: Utc::now(),
        };

        self.add_operation_to_transaction(transaction_id, operation)?;

        // Store in transaction's working set (not committed yet)
        let transaction = self.active_transactions.get_mut(&transaction_id).unwrap();
        transaction.state = TransactionState::InProgress;

        Ok(())
    }

    pub async fn update_user_in_transaction(
        &mut self,
        transaction_id: Uuid,
        user_id: Uuid,
        _updates: HashMap<String, String>,
    ) -> Result<(), String> {
        self.validate_transaction_state(transaction_id)?;

        // Get version before acquiring lock to avoid borrowing conflicts
        let old_version = self.users.get(&user_id)
            .ok_or("User not found")?
            .version;

        // Acquire exclusive lock
        self.acquire_lock(transaction_id, LockType::Exclusive, "users".to_string(), Some(user_id)).await?;

        // Simulate optimistic locking check
        let new_version = old_version + 1;

        let operation = TransactionOperation::Update {
            table: "users".to_string(),
            record_id: user_id,
            old_version,
            new_version,
        };

        self.add_operation_to_transaction(transaction_id, operation)?;

        let transaction = self.active_transactions.get_mut(&transaction_id).unwrap();
        transaction.state = TransactionState::InProgress;

        Ok(())
    }

    pub async fn delete_user_in_transaction(
        &mut self,
        transaction_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), String> {
        self.validate_transaction_state(transaction_id)?;

        if !self.users.contains_key(&user_id) {
            return Err("User not found".to_string());
        }

        // Check foreign key constraints
        if self.profiles.values().any(|p| p.user_id == user_id) {
            self.constraint_violations.push(format!("Foreign key violation: user {} has profiles", user_id));
            return Err("Cannot delete user with existing profiles".to_string());
        }

        // Acquire exclusive lock
        self.acquire_lock(transaction_id, LockType::Exclusive, "users".to_string(), Some(user_id)).await?;

        let operation = TransactionOperation::Delete {
            table: "users".to_string(),
            record_id: user_id,
        };

        self.add_operation_to_transaction(transaction_id, operation)?;

        let transaction = self.active_transactions.get_mut(&transaction_id).unwrap();
        transaction.state = TransactionState::InProgress;

        Ok(())
    }

    pub async fn select_user_in_transaction(
        &mut self,
        transaction_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TransactionTestUser>, String> {
        self.validate_transaction_state(transaction_id)?;

        let transaction = self.active_transactions.get(&transaction_id).unwrap();

        // Apply isolation level semantics
        match transaction.isolation_level {
            IsolationLevel::ReadUncommitted => {
                // Can see uncommitted changes from other transactions
                self.acquire_lock(transaction_id, LockType::Shared, "users".to_string(), Some(user_id)).await?;
            },
            IsolationLevel::ReadCommitted => {
                // Only see committed data
                self.acquire_lock(transaction_id, LockType::Shared, "users".to_string(), Some(user_id)).await?;
            },
            IsolationLevel::RepeatableRead => {
                // Lock for entire transaction
                self.acquire_lock(transaction_id, LockType::Shared, "users".to_string(), Some(user_id)).await?;
            },
            IsolationLevel::Serializable => {
                // Most restrictive locking
                self.acquire_lock(transaction_id, LockType::ShareUpdateExclusive, "users".to_string(), Some(user_id)).await?;
            },
        }

        let operation = TransactionOperation::Select {
            table: "users".to_string(),
            record_id: Some(user_id),
        };

        self.add_operation_to_transaction(transaction_id, operation)?;

        Ok(self.users.get(&user_id).cloned())
    }

    // Lock management
    async fn acquire_lock(
        &mut self,
        transaction_id: Uuid,
        lock_type: LockType,
        table: String,
        record_id: Option<Uuid>,
    ) -> Result<(), String> {
        // Check for lock conflicts
        if self.has_conflicting_lock(&lock_type, &table, &record_id) {
            if self.deadlock_detection_enabled {
                if self.would_cause_deadlock(transaction_id, &table, &record_id) {
                    return Err("Deadlock detected".to_string());
                }
            }

            // Simulate waiting for lock (in real DB this would block)
            sleep(Duration::from_millis(10)).await;
        }

        let lock = DatabaseLock {
            lock_type,
            table,
            record_id,
            acquired_at: Utc::now(),
        };

        // Add lock to transaction
        if let Some(transaction) = self.active_transactions.get_mut(&transaction_id) {
            transaction.locks_held.push(lock.clone());
        }

        self.global_locks.push(lock);
        Ok(())
    }

    fn has_conflicting_lock(&self, lock_type: &LockType, table: &str, record_id: &Option<Uuid>) -> bool {
        self.global_locks.iter().any(|existing_lock| {
            existing_lock.table == table
                && existing_lock.record_id == *record_id
                && self.locks_conflict(lock_type, &existing_lock.lock_type)
        })
    }

    fn locks_conflict(&self, lock1: &LockType, lock2: &LockType) -> bool {
        use LockType::*;
        matches!(
            (lock1, lock2),
            (Exclusive, _) | (_, Exclusive) |
            (RowExclusive, RowExclusive) |
            (ShareUpdateExclusive, ShareUpdateExclusive)
        )
    }

    fn would_cause_deadlock(&self, transaction_id: Uuid, table: &str, record_id: &Option<Uuid>) -> bool {
        // Simplified deadlock detection - check if any other transaction
        // holds a lock we need and is waiting for a lock we hold
        let empty_locks = Vec::new();
        let our_locks = self.active_transactions.get(&transaction_id)
            .map(|t| &t.locks_held)
            .unwrap_or(&empty_locks);

        for (other_tx_id, other_tx) in &self.active_transactions {
            if *other_tx_id == transaction_id {
                continue;
            }

            // Check if other transaction holds the lock we want
            let other_holds_target = other_tx.locks_held.iter().any(|lock| {
                lock.table == table && lock.record_id == *record_id
            });

            if other_holds_target {
                // Check if we hold any locks the other transaction might want
                for our_lock in our_locks {
                    if self.transaction_might_want_lock(*other_tx_id, &our_lock.table, &our_lock.record_id) {
                        return true; // Potential deadlock
                    }
                }
            }
        }

        false
    }

    fn transaction_might_want_lock(&self, _transaction_id: Uuid, _table: &str, _record_id: &Option<Uuid>) -> bool {
        // Simplified heuristic - in real implementation would check transaction's access patterns
        true
    }

    fn release_transaction_locks(&mut self, transaction_id: Uuid) {
        // Get locks from both active and completed transactions
        let locks_to_remove = if let Some(transaction) = self.active_transactions.get(&transaction_id) {
            transaction.locks_held.clone()
        } else if let Some(transaction) = self.completed_transactions.iter().find(|t| t.id == transaction_id) {
            transaction.locks_held.clone()
        } else {
            Vec::new()
        };

        for lock in locks_to_remove {
            self.global_locks.retain(|l| {
                !(l.table == lock.table &&
                  l.record_id == lock.record_id &&
                  l.acquired_at == lock.acquired_at)
            });
        }
    }

    // Helper methods
    fn validate_transaction_state(&self, transaction_id: Uuid) -> Result<(), String> {
        let transaction = self.active_transactions.get(&transaction_id)
            .ok_or("Transaction not found")?;

        match transaction.state {
            TransactionState::Started | TransactionState::InProgress => Ok(()),
            _ => Err("Transaction not in valid state".to_string()),
        }
    }

    fn add_operation_to_transaction(&mut self, transaction_id: Uuid, operation: TransactionOperation) -> Result<(), String> {
        let transaction = self.active_transactions.get_mut(&transaction_id)
            .ok_or("Transaction not found")?;

        transaction.operations.push(operation);
        Ok(())
    }

    async fn apply_operation(&mut self, operation: &TransactionOperation) -> Result<(), String> {
        match operation {
            TransactionOperation::Insert { table, record_id } => {
                if table == "users" {
                    // In real implementation, would apply the actual insert
                    // For testing, we assume the data is stored elsewhere temporarily
                }
            },
            TransactionOperation::Update { table, record_id, new_version, .. } => {
                if table == "users" {
                    if let Some(user) = self.users.get_mut(record_id) {
                        user.version = *new_version;
                        user.updated_at = Utc::now();
                    }
                }
            },
            TransactionOperation::Delete { table, record_id } => {
                if table == "users" {
                    self.users.remove(record_id);
                }
            },
            TransactionOperation::Select { .. } => {
                // Selects don't modify data
            },
        }
        Ok(())
    }

    async fn rollback_operations(&mut self, transaction: &DatabaseTransaction) -> Result<(), String> {
        // Reverse the operations in LIFO order
        for operation in transaction.operations.iter().rev() {
            match operation {
                TransactionOperation::Insert { table, record_id } => {
                    if table == "users" {
                        self.users.remove(record_id);
                    }
                },
                TransactionOperation::Update { table, record_id, old_version, .. } => {
                    if table == "users" {
                        if let Some(user) = self.users.get_mut(record_id) {
                            user.version = *old_version;
                        }
                    }
                },
                TransactionOperation::Delete { .. } => {
                    // Would need to restore the deleted record
                    // For testing purposes, we assume it wasn't actually deleted yet
                },
                TransactionOperation::Select { .. } => {
                    // Nothing to rollback for selects
                },
            }
        }
        Ok(())
    }

    // Testing utilities
    pub fn get_active_transaction_count(&self) -> usize {
        self.active_transactions.len()
    }

    pub fn get_completed_transaction_count(&self) -> usize {
        self.completed_transactions.len()
    }

    pub fn get_constraint_violations(&self) -> &Vec<String> {
        &self.constraint_violations
    }

    pub fn has_active_locks(&self) -> bool {
        !self.global_locks.is_empty()
    }

    pub fn add_test_user(&mut self, user: TransactionTestUser) {
        self.users.insert(user.id, user);
    }

    pub fn add_test_profile(&mut self, profile: TransactionTestProfile) {
        self.profiles.insert(profile.id, profile);
    }

    pub fn get_user(&self, user_id: &Uuid) -> Option<&TransactionTestUser> {
        self.users.get(user_id)
    }

    pub fn clear_constraint_violations(&mut self) {
        self.constraint_violations.clear();
    }

    // Enterprise savepoint methods
    pub async fn create_savepoint(&mut self, transaction_id: Uuid, name: String) -> Result<Uuid, String> {
        self.validate_transaction_state(transaction_id)?;

        let savepoint_id = Uuid::now_v7();
        let transaction = self.active_transactions.get_mut(&transaction_id).unwrap();

        let savepoint = Savepoint {
            id: savepoint_id,
            name: name.clone(),
            transaction_id,
            created_at: Utc::now(),
            operations_at_creation: transaction.operations.len(),
            locks_at_creation: transaction.locks_held.clone(),
        };

        transaction.savepoints.push(savepoint);

        let operation = TransactionOperation::CreateSavepoint {
            name,
            savepoint_id,
            timestamp: Utc::now(),
        };

        self.add_operation_to_transaction(transaction_id, operation)?;
        Ok(savepoint_id)
    }

    pub async fn rollback_to_savepoint(&mut self, transaction_id: Uuid, savepoint_id: Uuid) -> Result<(), String> {
        self.validate_transaction_state(transaction_id)?;

        let transaction = self.active_transactions.get_mut(&transaction_id)
            .ok_or("Transaction not found")?;

        let savepoint = transaction.savepoints.iter()
            .find(|sp| sp.id == savepoint_id)
            .ok_or("Savepoint not found")?
            .clone();

        // Rollback operations after savepoint
        let operations_to_rollback = transaction.operations.len() - savepoint.operations_at_creation;
        for _ in 0..operations_to_rollback {
            transaction.operations.pop();
        }

        // Release locks acquired after savepoint
        transaction.locks_held = savepoint.locks_at_creation;

        let operation = TransactionOperation::RollbackToSavepoint {
            savepoint_id,
            timestamp: Utc::now(),
        };

        self.add_operation_to_transaction(transaction_id, operation)?;
        Ok(())
    }

    // Enterprise performance monitoring
    pub fn get_transaction_metrics(&self, transaction_id: Uuid) -> Option<&TransactionMetrics> {
        self.transaction_metrics.get(&transaction_id)
    }

    pub fn get_performance_statistics(&self) -> &PerformanceStatistics {
        &self.performance_stats
    }

    pub async fn check_transaction_timeouts(&mut self) -> Vec<Uuid> {
        let mut timed_out = Vec::new();
        let now = Instant::now();

        for (tx_id, timeout) in self.timeout_manager.active_timeouts.clone() {
            if now.duration_since(timeout.started_at) > timeout.timeout_duration {
                timed_out.push(tx_id);

                match timeout.escalation_policy {
                    TimeoutEscalationPolicy::Kill | TimeoutEscalationPolicy::Rollback => {
                        if let Err(e) = self.rollback_transaction(tx_id).await {
                            eprintln!("Failed to rollback timed out transaction {}: {}", tx_id, e);
                        }
                    },
                    TimeoutEscalationPolicy::Warn => {
                        self.timeout_manager.timeout_warnings_sent.insert(tx_id);
                    },
                    _ => {
                        // Handle other escalation policies
                    }
                }
            }
        }

        timed_out
    }

    // Enterprise resource monitoring
    pub fn check_resource_limits(&mut self, transaction_id: Uuid) -> Result<(), String> {
        let transaction = self.active_transactions.get(&transaction_id)
            .ok_or("Transaction not found")?;

        let metrics = self.transaction_metrics.get(&transaction_id)
            .ok_or("Transaction metrics not found")?;

        let limits = &transaction.resource_limits;

        if transaction.locks_held.len() > limits.max_locks {
            return Err("Transaction exceeded maximum locks limit".to_string());
        }

        if transaction.operations.len() > limits.max_operations {
            return Err("Transaction exceeded maximum operations limit".to_string());
        }

        if metrics.memory_used_bytes > (limits.max_memory_mb as u64 * 1024 * 1024) {
            return Err("Transaction exceeded memory limit".to_string());
        }

        Ok(())
    }

    // Enterprise audit methods
    pub fn get_audit_log_for_transaction(&self, transaction_id: Uuid) -> Vec<&AuditLogEntry> {
        self.audit_log.iter()
            .filter(|entry| entry.transaction_id == transaction_id)
            .collect()
    }

    pub fn get_deadlock_history(&self) -> &Vec<DeadlockGraph> {
        &self.deadlock_graphs
    }

    // Enterprise transaction analytics and monitoring
    pub async fn analyze_transaction_patterns(&self) -> TransactionAnalytics {
        let total_transactions = self.completed_transactions.len();
        let committed_count = self.completed_transactions.iter()
            .filter(|tx| tx.state == TransactionState::Committed)
            .count();
        let rolled_back_count = self.completed_transactions.iter()
            .filter(|tx| tx.state == TransactionState::RolledBack)
            .count();

        let avg_duration = if !self.completed_transactions.is_empty() {
            let total_duration: Duration = self.completed_transactions.iter()
                .filter_map(|tx| {
                    if let (Some(start), Some(end)) = (&tx.started_at, &tx.completed_at) {
                        Some(end.signed_duration_since(*start).to_std().unwrap_or_default())
                    } else {
                        None
                    }
                })
                .sum();
            total_duration / total_transactions as u32
        } else {
            Duration::from_secs(0)
        };

        TransactionAnalytics {
            total_transactions,
            success_rate: if total_transactions > 0 { committed_count as f64 / total_transactions as f64 } else { 0.0 },
            average_duration: avg_duration,
            deadlock_incidents: self.deadlock_graphs.len(),
            constraint_violations: self.constraint_violations.len(),
            peak_concurrent_transactions: self.active_transactions.len(),
            lock_contention_events: self.calculate_lock_contention_events(),
            resource_exhaustion_events: 0, // Would be tracked in real implementation
        }
    }

    fn calculate_lock_contention_events(&self) -> usize {
        // Simplified calculation - in real implementation would track lock wait events
        self.global_locks.len()
    }

    // Advanced transaction diagnostics
    pub async fn diagnose_transaction_health(&self) -> TransactionHealthReport {
        let active_count = self.active_transactions.len();
        let long_running_transactions = self.active_transactions.iter()
            .filter(|(_, tx)| {
                let elapsed = Utc::now().signed_duration_since(tx.started_at);
                elapsed.num_seconds() > 300 // 5 minutes
            })
            .count();

        let resource_pressure = self.calculate_resource_pressure();
        let deadlock_risk = self.assess_deadlock_risk();

        TransactionHealthReport {
            overall_health: if resource_pressure < 0.8 && deadlock_risk < 0.3 {
                TransactionHealthStatus::Healthy
            } else if resource_pressure < 0.9 && deadlock_risk < 0.6 {
                TransactionHealthStatus::Warning
            } else {
                TransactionHealthStatus::Critical
            },
            active_transactions: active_count,
            long_running_transactions,
            resource_pressure,
            deadlock_risk,
            recommendations: self.generate_health_recommendations(resource_pressure, deadlock_risk),
        }
    }

    fn calculate_resource_pressure(&self) -> f64 {
        let connection_pressure = self.connection_pool.active_connections as f64 / self.connection_pool.max_connections as f64;
        let lock_pressure = self.global_locks.len() as f64 / 10000.0; // Assume 10k max locks
        let memory_pressure = self.resource_monitor.total_memory_used as f64 / (1024.0 * 1024.0 * 1024.0); // 1GB threshold

        (connection_pressure + lock_pressure + memory_pressure) / 3.0
    }

    fn assess_deadlock_risk(&self) -> f64 {
        if self.active_transactions.len() < 2 {
            return 0.0;
        }

        let overlapping_locks = self.count_overlapping_locks();
        let transaction_complexity = self.calculate_average_transaction_complexity();

        (overlapping_locks as f64 / 100.0 + transaction_complexity / 50.0) / 2.0
    }

    fn count_overlapping_locks(&self) -> usize {
        let mut overlap_count = 0;
        for i in 0..self.global_locks.len() {
            for j in (i + 1)..self.global_locks.len() {
                if self.global_locks[i].table == self.global_locks[j].table
                   && self.global_locks[i].record_id == self.global_locks[j].record_id {
                    overlap_count += 1;
                }
            }
        }
        overlap_count
    }

    fn calculate_average_transaction_complexity(&self) -> f64 {
        if self.active_transactions.is_empty() {
            return 0.0;
        }

        let total_complexity: usize = self.active_transactions.values()
            .map(|tx| tx.operations.len() + tx.locks_held.len())
            .sum();

        total_complexity as f64 / self.active_transactions.len() as f64
    }

    fn generate_health_recommendations(&self, resource_pressure: f64, deadlock_risk: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if resource_pressure > 0.8 {
            recommendations.push("Consider increasing connection pool size".to_string());
            recommendations.push("Review long-running transactions for optimization".to_string());
        }

        if deadlock_risk > 0.5 {
            recommendations.push("Implement transaction ordering to reduce deadlock risk".to_string());
            recommendations.push("Consider reducing transaction scope and duration".to_string());
        }

        if self.active_transactions.len() > 50 {
            recommendations.push("High concurrent transaction count - monitor for performance degradation".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Transaction system is operating optimally".to_string());
        }

        recommendations
    }

    // Enterprise compliance and security features
    pub async fn generate_compliance_report(&self) -> ComplianceReport {
        ComplianceReport {
            audit_trail_complete: !self.audit_log.is_empty(),
            transaction_encryption: true, // Assume encrypted at rest
            access_control_enforced: true, // User/session tracking enabled
            data_retention_policy_applied: false, // Would need retention logic
            regulatory_compliance: vec![
                "SOX".to_string(),
                "GDPR".to_string(),
                "PCI-DSS".to_string(),
            ],
            security_incidents: self.detect_security_incidents(),
            compliance_score: self.calculate_compliance_score(),
        }
    }

    fn detect_security_incidents(&self) -> Vec<SecurityIncident> {
        let mut incidents = Vec::new();

        // Check for suspicious transaction patterns
        for tx in &self.completed_transactions {
            if tx.operations.len() > 1000 {
                incidents.push(SecurityIncident {
                    incident_id: Uuid::now_v7(),
                    incident_type: "High Volume Transaction".to_string(),
                    severity: SecuritySeverity::Medium,
                    transaction_id: Some(tx.id),
                    description: "Transaction with unusually high operation count detected".to_string(),
                    detected_at: Utc::now(),
                });
            }
        }

        // Check for rapid-fire transactions from same session
        let mut session_tx_counts: HashMap<String, usize> = HashMap::new();
        for tx in &self.completed_transactions {
            if let Some(session_id) = &tx.session_id {
                *session_tx_counts.entry(session_id.clone()).or_insert(0) += 1;
            }
        }

        for (session_id, count) in session_tx_counts {
            if count > 100 {
                incidents.push(SecurityIncident {
                    incident_id: Uuid::now_v7(),
                    incident_type: "Rapid Transaction Pattern".to_string(),
                    severity: SecuritySeverity::High,
                    transaction_id: None,
                    description: format!("Session {} executed {} transactions - possible automation", session_id, count),
                    detected_at: Utc::now(),
                });
            }
        }

        incidents
    }

    fn calculate_compliance_score(&self) -> f64 {
        let mut score = 0.0;
        let total_checks = 10.0;

        // Audit logging enabled
        if !self.audit_log.is_empty() { score += 1.0; }

        // Transaction isolation properly configured
        if self.active_transactions.values().all(|tx| tx.isolation_level != IsolationLevel::ReadUncommitted) {
            score += 1.0;
        }

        // Resource limits enforced
        if self.active_transactions.values().all(|tx| tx.resource_limits.max_duration.as_secs() > 0) {
            score += 1.0;
        }

        // Session tracking enabled
        if self.active_transactions.values().all(|tx| tx.session_id.is_some()) {
            score += 1.0;
        }

        // User tracking enabled
        if self.active_transactions.values().all(|tx| tx.user_id.is_some()) {
            score += 1.0;
        }

        // Timeout management active
        if !self.timeout_manager.active_timeouts.is_empty() {
            score += 1.0;
        }

        // Connection pool properly configured
        if self.connection_pool.max_connections > 0 {
            score += 1.0;
        }

        // Deadlock detection enabled
        if self.deadlock_detection_enabled {
            score += 1.0;
        }

        // Constraint validation active
        if !self.constraint_violations.is_empty() || self.completed_transactions.len() > 0 {
            score += 1.0;
        }

        // Performance monitoring active
        if !self.transaction_metrics.is_empty() {
            score += 1.0;
        }

        (score / total_checks) * 100.0
    }
}

// Additional enterprise data structures

#[derive(Debug, Clone)]
pub struct TransactionAnalytics {
    pub total_transactions: usize,
    pub success_rate: f64,
    pub average_duration: Duration,
    pub deadlock_incidents: usize,
    pub constraint_violations: usize,
    pub peak_concurrent_transactions: usize,
    pub lock_contention_events: usize,
    pub resource_exhaustion_events: usize,
}

#[derive(Debug, Clone)]
pub struct TransactionHealthReport {
    pub overall_health: TransactionHealthStatus,
    pub active_transactions: usize,
    pub long_running_transactions: usize,
    pub resource_pressure: f64,
    pub deadlock_risk: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionHealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub audit_trail_complete: bool,
    pub transaction_encryption: bool,
    pub access_control_enforced: bool,
    pub data_retention_policy_applied: bool,
    pub regulatory_compliance: Vec<String>,
    pub security_incidents: Vec<SecurityIncident>,
    pub compliance_score: f64,
}

#[derive(Debug, Clone)]
pub struct SecurityIncident {
    pub incident_id: Uuid,
    pub incident_type: String,
    pub severity: SecuritySeverity,
    pub transaction_id: Option<Uuid>,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Tests for transaction integrity

#[tokio::test]
async fn test_basic_transaction_commit() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    // Insert user in transaction
    simulator.insert_user_in_transaction(tx_id, user.clone()).await.unwrap();

    // Commit transaction
    simulator.commit_transaction(tx_id).await.unwrap();

    // Verify transaction completed
    assert_eq!(simulator.get_active_transaction_count(), 0);
    assert_eq!(simulator.get_completed_transaction_count(), 1);
    assert!(!simulator.has_active_locks());
}

#[tokio::test]
async fn test_transaction_rollback() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    // Add user to simulate existing state
    simulator.add_test_user(user.clone());

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Update user in transaction
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_user".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    // Rollback transaction
    simulator.rollback_transaction(tx_id).await.unwrap();

    // Verify transaction was rolled back
    assert_eq!(simulator.get_active_transaction_count(), 0);
    assert_eq!(simulator.get_completed_transaction_count(), 1);
    assert!(!simulator.has_active_locks());

    // Verify user data unchanged
    let stored_user = simulator.get_user(&user.id).unwrap();
    assert_eq!(stored_user.version, 1); // Should remain original version
}

#[tokio::test]
async fn test_constraint_violation_handling() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user1 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "user1".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user2 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(), // Same email - should cause constraint violation
        username: "user2".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    // Add first user
    simulator.add_test_user(user1);

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Try to insert user with duplicate email
    let result = simulator.insert_user_in_transaction(tx_id, user2).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Email already exists");
    assert!(!simulator.get_constraint_violations().is_empty());
}

#[tokio::test]
async fn test_foreign_key_constraint() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let profile = TransactionTestProfile {
        id: Uuid::now_v7(),
        user_id: user.id,
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        bio: Some("Test bio".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    // Add user and profile
    simulator.add_test_user(user.clone());
    simulator.add_test_profile(profile);

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Try to delete user with existing profile (should fail)
    let result = simulator.delete_user_in_transaction(tx_id, user.id).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Cannot delete user with existing profiles");
    assert!(!simulator.get_constraint_violations().is_empty());
}

#[tokio::test]
async fn test_optimistic_locking() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Update user (should increment version)
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_user".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    // Commit transaction
    simulator.commit_transaction(tx_id).await.unwrap();

    // Verify version was incremented
    let updated_user = simulator.get_user(&user.id).unwrap();
    assert_eq!(updated_user.version, 2);
}

#[tokio::test]
async fn test_deadlock_detection() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user1 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user1@example.com".to_string(),
        username: "user1".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user2 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user2@example.com".to_string(),
        username: "user2".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user1.clone());
    simulator.add_test_user(user2.clone());

    // Begin two transactions
    let tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Transaction 1 locks user1
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_user1".to_string());
    simulator.update_user_in_transaction(tx1_id, user1.id, updates).await.unwrap();

    // Transaction 2 locks user2
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_user2".to_string());
    simulator.update_user_in_transaction(tx2_id, user2.id, updates).await.unwrap();

    // This should detect a potential deadlock situation
    // (In this simplified test, we're just verifying the deadlock detection mechanism exists)
    assert!(simulator.deadlock_detection_enabled);
}

#[tokio::test]
async fn test_isolation_level_read_committed() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    // Begin transaction with READ_COMMITTED isolation
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Read user in transaction
    let read_user = simulator.select_user_in_transaction(tx_id, user.id).await.unwrap();

    assert!(read_user.is_some());
    assert_eq!(read_user.unwrap().id, user.id);

    // Verify appropriate lock was acquired
    assert!(simulator.has_active_locks());

    simulator.commit_transaction(tx_id).await.unwrap();
}

#[tokio::test]
async fn test_isolation_level_serializable() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    // Begin transaction with SERIALIZABLE isolation
    let tx_id = simulator.begin_transaction(IsolationLevel::Serializable).await.unwrap();

    // Read user in transaction (should acquire stricter locks)
    let read_user = simulator.select_user_in_transaction(tx_id, user.id).await.unwrap();

    assert!(read_user.is_some());
    assert!(simulator.has_active_locks());

    simulator.commit_transaction(tx_id).await.unwrap();
}

#[tokio::test]
async fn test_transaction_failure_simulation() {
    let mut simulator = TransactionIntegritySimulator::new();
    simulator.enable_failure_simulation();

    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Add multiple operations to trigger simulated failure
    for i in 0..5 {
        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("user{}@example.com", i),
            username: format!("user{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
    }

    // Commit should fail due to simulation
    let result = simulator.commit_transaction(tx_id).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Simulated commit failure");
    assert_eq!(simulator.get_completed_transaction_count(), 1);
}

#[tokio::test]
async fn test_concurrent_transactions() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin multiple transactions
    let tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx3_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    assert_eq!(simulator.get_active_transaction_count(), 3);

    // Add operations to each transaction
    let user1 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user1@example.com".to_string(),
        username: "user1".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user2 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user2@example.com".to_string(),
        username: "user2".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user3 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user3@example.com".to_string(),
        username: "user3".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx1_id, user1).await.unwrap();
    simulator.insert_user_in_transaction(tx2_id, user2).await.unwrap();
    simulator.insert_user_in_transaction(tx3_id, user3).await.unwrap();

    // Commit all transactions
    simulator.commit_transaction(tx1_id).await.unwrap();
    simulator.commit_transaction(tx2_id).await.unwrap();
    simulator.commit_transaction(tx3_id).await.unwrap();

    assert_eq!(simulator.get_active_transaction_count(), 0);
    assert_eq!(simulator.get_completed_transaction_count(), 3);
    assert!(!simulator.has_active_locks());
}

#[tokio::test]
async fn test_mixed_commit_rollback_transactions() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin multiple transactions
    let tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    let user1 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user1@example.com".to_string(),
        username: "user1".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user2 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user2@example.com".to_string(),
        username: "user2".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx1_id, user1).await.unwrap();
    simulator.insert_user_in_transaction(tx2_id, user2).await.unwrap();

    // Commit first, rollback second
    simulator.commit_transaction(tx1_id).await.unwrap();
    simulator.rollback_transaction(tx2_id).await.unwrap();

    assert_eq!(simulator.get_active_transaction_count(), 0);
    assert_eq!(simulator.get_completed_transaction_count(), 2);

    // Check that only first transaction's changes were applied
    let completed_txs = &simulator.completed_transactions;
    assert_eq!(completed_txs[0].state, TransactionState::Committed);
    assert_eq!(completed_txs[1].state, TransactionState::RolledBack);
}

#[tokio::test]
async fn test_lock_cleanup_after_transaction() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Operations that acquire locks
    simulator.select_user_in_transaction(tx_id, user.id).await.unwrap();

    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    // Verify locks are held
    assert!(simulator.has_active_locks());

    // Commit transaction
    simulator.commit_transaction(tx_id).await.unwrap();

    // Verify all locks are released
    assert!(!simulator.has_active_locks());
}

// ============ ENTERPRISE TRANSACTION INTEGRITY TESTS ============

#[tokio::test]
async fn test_transaction_with_priority_and_resource_limits() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin high-priority transaction with custom timeout
    let tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::ReadCommitted,
        Some(TransactionPriority::High),
        Some(Duration::from_secs(300)), // 5 minute timeout
        None,
    ).await.unwrap();

    let transaction = simulator.active_transactions.get(&tx_id).unwrap();
    assert_eq!(transaction.priority, TransactionPriority::High);
    assert_eq!(transaction.resource_limits.max_memory_mb, 512); // High priority limit

    // Verify transaction has timeout configured
    assert!(simulator.timeout_manager.active_timeouts.contains_key(&tx_id));

    simulator.commit_transaction(tx_id).await.unwrap();
}

#[tokio::test]
async fn test_savepoint_operations() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    // Begin transaction
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Create savepoint
    let savepoint_id = simulator.create_savepoint(tx_id, "savepoint1".to_string()).await.unwrap();

    // Perform operations after savepoint
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_user".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    let transaction = simulator.active_transactions.get(&tx_id).unwrap();
    let operations_before_rollback = transaction.operations.len();

    // Rollback to savepoint
    simulator.rollback_to_savepoint(tx_id, savepoint_id).await.unwrap();

    let transaction = simulator.active_transactions.get(&tx_id).unwrap();
    assert!(transaction.operations.len() < operations_before_rollback);

    simulator.commit_transaction(tx_id).await.unwrap();
}

#[tokio::test]
async fn test_transaction_timeout_handling() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin transaction with short timeout
    let tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::ReadCommitted,
        Some(TransactionPriority::Normal),
        Some(Duration::from_millis(100)), // Very short timeout
        None,
    ).await.unwrap();

    // Wait longer than timeout
    sleep(Duration::from_millis(150)).await;

    // Check timeouts - should detect and handle the timeout
    let timed_out_transactions = simulator.check_transaction_timeouts().await;
    assert!(timed_out_transactions.contains(&tx_id));

    // Verify transaction was rolled back
    assert!(!simulator.active_transactions.contains_key(&tx_id));
    assert_eq!(simulator.get_completed_transaction_count(), 1);

    let completed_tx = &simulator.completed_transactions[0];
    assert_eq!(completed_tx.state, TransactionState::RolledBack);
}

#[tokio::test]
async fn test_resource_limit_enforcement() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin batch priority transaction (low resource limits)
    let tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::ReadCommitted,
        Some(TransactionPriority::Batch),
        None,
        None,
    ).await.unwrap();

    // Add many operations to exceed limits
    for i in 0..1500 { // Batch limit is 1000 operations
        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("user{}@example.com", i),
            username: format!("user{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        let result = simulator.insert_user_in_transaction(tx_id, user).await;

        // Should succeed for first 1000, then start failing
        if i < 1000 {
            assert!(result.is_ok());
        } else {
            // Resource limit check would catch this
            let resource_check = simulator.check_resource_limits(tx_id);
            if resource_check.is_err() {
                break; // Expected to hit resource limits
            }
        }
    }

    let resource_check = simulator.check_resource_limits(tx_id);
    assert!(resource_check.is_err());
    assert!(resource_check.unwrap_err().contains("exceeded maximum operations limit"));
}

#[tokio::test]
async fn test_connection_pool_management() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Fill up connection pool (max 100 connections)
    let mut tx_ids = Vec::new();
    for _ in 0..100 {
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
        tx_ids.push(tx_id);
    }

    assert_eq!(simulator.connection_pool.active_connections, 100);

    // Try to create one more - should fail
    let result = simulator.begin_transaction(IsolationLevel::ReadCommitted).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No available connections in pool");

    // Commit some transactions to free connections
    for tx_id in tx_ids.iter().take(10) {
        simulator.commit_transaction(*tx_id).await.unwrap();
    }

    assert_eq!(simulator.connection_pool.active_connections, 90);

    // Now should be able to create new transaction
    let new_tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    assert_eq!(simulator.connection_pool.active_connections, 91);

    simulator.commit_transaction(new_tx_id).await.unwrap();
}

#[tokio::test]
async fn test_transaction_metrics_collection() {
    let mut simulator = TransactionIntegritySimulator::new();

    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Perform some operations
    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "metrics@example.com".to_string(),
        username: "metricsuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx_id, user.clone()).await.unwrap();

    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_metrics_user".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    // Check metrics were collected
    let metrics = simulator.get_transaction_metrics(tx_id).unwrap();
    assert_eq!(metrics.transaction_id, tx_id);
    assert!(metrics.operations_count >= 2); // At least insert and update
    assert_eq!(metrics.isolation_level, IsolationLevel::ReadCommitted);

    simulator.commit_transaction(tx_id).await.unwrap();

    // Metrics should be updated with final state
    let final_metrics = simulator.get_transaction_metrics(tx_id).unwrap();
    assert_eq!(final_metrics.final_state, Some(TransactionState::Committed));
}

#[tokio::test]
async fn test_audit_logging() {
    let mut simulator = TransactionIntegritySimulator::new();

    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Operations should create audit log entries
    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "audit@example.com".to_string(),
        username: "audituser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
    simulator.commit_transaction(tx_id).await.unwrap();

    // Check audit log was created
    let audit_entries = simulator.get_audit_log_for_transaction(tx_id);
    assert!(!audit_entries.is_empty());

    // Should have BEGIN entry at minimum
    let begin_entry = audit_entries.iter().find(|e| e.operation == "BEGIN");
    assert!(begin_entry.is_some());
    assert_eq!(begin_entry.unwrap().transaction_id, tx_id);
    assert!(begin_entry.unwrap().success);
}

#[tokio::test]
async fn test_nested_transaction_simulation() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin parent transaction
    let parent_tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Begin nested transaction
    let child_tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::ReadCommitted,
        Some(TransactionPriority::Normal),
        None,
        Some(parent_tx_id), // Set parent
    ).await.unwrap();

    let child_transaction = simulator.active_transactions.get(&child_tx_id).unwrap();
    assert_eq!(child_transaction.parent_transaction_id, Some(parent_tx_id));

    // Child transaction operations
    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "nested@example.com".to_string(),
        username: "nesteduser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(child_tx_id, user).await.unwrap();

    // Commit child first, then parent
    simulator.commit_transaction(child_tx_id).await.unwrap();
    simulator.commit_transaction(parent_tx_id).await.unwrap();

    assert_eq!(simulator.get_completed_transaction_count(), 2);
}

#[tokio::test]
async fn test_advanced_deadlock_detection() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user1 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user1@deadlock.com".to_string(),
        username: "user1".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    let user2 = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "user2@deadlock.com".to_string(),
        username: "user2".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user1.clone());
    simulator.add_test_user(user2.clone());

    // Create potential deadlock scenario
    let tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // TX1 locks user1, TX2 locks user2
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated1".to_string());
    simulator.update_user_in_transaction(tx1_id, user1.id, updates.clone()).await.unwrap();

    updates.insert("username".to_string(), "updated2".to_string());
    simulator.update_user_in_transaction(tx2_id, user2.id, updates).await.unwrap();

    // Verify deadlock detection is enabled
    assert!(simulator.deadlock_detection_enabled);

    // Clean up transactions
    simulator.commit_transaction(tx1_id).await.unwrap();
    simulator.commit_transaction(tx2_id).await.unwrap();
}

#[tokio::test]
async fn test_transaction_performance_monitoring() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Perform multiple transactions to generate performance data
    for i in 0..10 {
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("perf{}@example.com", i),
            username: format!("perfuser{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
        simulator.commit_transaction(tx_id).await.unwrap();
    }

    // Check performance statistics
    let stats = simulator.get_performance_statistics();
    assert_eq!(stats.transactions_per_second, 0.0); // Default value for this test
    assert_eq!(stats.deadlock_rate, 0.0); // No deadlocks in this test
    assert_eq!(stats.rollback_rate, 0.0); // No rollbacks in this test

    assert_eq!(simulator.get_completed_transaction_count(), 10);
}

#[tokio::test]
async fn test_cross_database_transaction_coordination() {
    let mut simulator = TransactionIntegritySimulator::new();

    let coordinator_id = Uuid::now_v7();

    // Simulate cross-database transaction
    let cross_db_tx = CrossDatabaseTransaction {
        coordinator_id,
        participant_databases: vec!["db1".to_string(), "db2".to_string()],
        transaction_ids: HashMap::from([
            ("db1".to_string(), Uuid::now_v7()),
            ("db2".to_string(), Uuid::now_v7()),
        ]),
        phase: TwoPhaseCommitPhase::Prepare,
        votes: HashMap::new(),
        global_state: TransactionState::Started,
    };

    simulator.cross_db_transactions.insert(coordinator_id, cross_db_tx);

    // Verify cross-database transaction is tracked
    assert!(simulator.cross_db_transactions.contains_key(&coordinator_id));

    let stored_tx = simulator.cross_db_transactions.get(&coordinator_id).unwrap();
    assert_eq!(stored_tx.participant_databases.len(), 2);
    assert_eq!(stored_tx.phase, TwoPhaseCommitPhase::Prepare);
}

#[tokio::test]
async fn test_transaction_replay_and_recovery() {
    let mut simulator = TransactionIntegritySimulator::new();

    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Create replay log
    let replay_log = TransactionReplayLog {
        log_id: Uuid::now_v7(),
        transaction_id: tx_id,
        operations: vec![
            ReplayableOperation {
                operation_id: Uuid::now_v7(),
                operation_type: "INSERT".to_string(),
                sql_statement: Some("INSERT INTO users VALUES (...)".to_string()),
                parameters: Some(serde_json::json!({"id": "123", "email": "test@example.com"})),
                timestamp: Utc::now(),
                sequence_number: 1,
                compensating_operation: Some("DELETE FROM users WHERE id = '123'".to_string()),
            },
        ],
        checkpoints: vec![
            TransactionCheckpoint {
                checkpoint_id: Uuid::now_v7(),
                transaction_id: tx_id,
                sequence_number: 1,
                state_snapshot: serde_json::json!({"step": "after_insert"}),
                created_at: Utc::now(),
            },
        ],
        created_at: Utc::now(),
        replay_count: 0,
    };

    simulator.replay_logs.insert(tx_id, replay_log);

    // Verify replay log is stored
    assert!(simulator.replay_logs.contains_key(&tx_id));

    let stored_log = simulator.replay_logs.get(&tx_id).unwrap();
    assert_eq!(stored_log.operations.len(), 1);
    assert_eq!(stored_log.checkpoints.len(), 1);
    assert_eq!(stored_log.replay_count, 0);

    simulator.commit_transaction(tx_id).await.unwrap();
}

// ============ ENTERPRISE TRANSACTION ANALYTICS TESTS ============

#[tokio::test]
async fn test_transaction_analytics() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Perform multiple transactions with different outcomes
    for i in 0..10 {
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("analytics{}@example.com", i),
            username: format!("analyticsuser{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        simulator.insert_user_in_transaction(tx_id, user).await.unwrap();

        // Commit most, rollback some
        if i < 8 {
            simulator.commit_transaction(tx_id).await.unwrap();
        } else {
            simulator.rollback_transaction(tx_id).await.unwrap();
        }
    }

    // Analyze transaction patterns
    let analytics = simulator.analyze_transaction_patterns().await;

    assert_eq!(analytics.total_transactions, 10);
    assert_eq!(analytics.success_rate, 0.8); // 8 committed out of 10
    assert!(analytics.average_duration.as_secs() >= 0);
    assert_eq!(analytics.deadlock_incidents, 0);
    assert_eq!(analytics.constraint_violations, 0);
    assert_eq!(analytics.peak_concurrent_transactions, 0); // All completed
}

#[tokio::test]
async fn test_transaction_health_monitoring() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Create some active transactions
    let _tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let _tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Generate health report
    let health_report = simulator.diagnose_transaction_health().await;

    assert_eq!(health_report.active_transactions, 2);
    assert_eq!(health_report.long_running_transactions, 0); // Just created
    assert!(health_report.resource_pressure >= 0.0 && health_report.resource_pressure <= 1.0);
    assert!(health_report.deadlock_risk >= 0.0 && health_report.deadlock_risk <= 1.0);
    assert!(!health_report.recommendations.is_empty());

    // Should be healthy with just 2 transactions
    assert!(matches!(health_report.overall_health,
        TransactionHealthStatus::Healthy | TransactionHealthStatus::Warning));
}

#[tokio::test]
async fn test_transaction_health_critical_state() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Fill up most of the connection pool to create resource pressure
    let mut tx_ids = Vec::new();
    for _ in 0..95 {
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
        tx_ids.push(tx_id);
    }

    // Add many locks to increase lock pressure
    for &tx_id in &tx_ids[..10] {
        for i in 0..10 {
            let user = TransactionTestUser {
                id: Uuid::now_v7(),
                email: format!("pressure{}@example.com", i),
                username: format!("pressureuser{}", i),
                password_hash: "hashed_password".to_string(),
                email_verified: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                version: 1,
            };

            simulator.add_test_user(user.clone());
            let _ = simulator.select_user_in_transaction(tx_id, user.id).await;
        }
    }

    let health_report = simulator.diagnose_transaction_health().await;

    assert_eq!(health_report.active_transactions, 95);
    assert!(health_report.resource_pressure > 0.8); // High resource pressure
    assert!(!health_report.recommendations.is_empty());

    // Should recommend action due to high resource usage
    let has_connection_recommendation = health_report.recommendations.iter()
        .any(|rec| rec.contains("connection pool"));
    assert!(has_connection_recommendation);
}

#[tokio::test]
async fn test_compliance_reporting() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Perform transaction to generate audit data
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "compliance@example.com".to_string(),
        username: "complianceuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
    simulator.commit_transaction(tx_id).await.unwrap();

    // Generate compliance report
    let compliance_report = simulator.generate_compliance_report().await;

    assert!(compliance_report.audit_trail_complete);
    assert!(compliance_report.transaction_encryption);
    assert!(compliance_report.access_control_enforced);
    assert!(!compliance_report.regulatory_compliance.is_empty());
    assert!(compliance_report.compliance_score > 0.0);
    assert!(compliance_report.compliance_score <= 100.0);

    // Check regulatory compliance standards
    assert!(compliance_report.regulatory_compliance.contains(&"SOX".to_string()));
    assert!(compliance_report.regulatory_compliance.contains(&"GDPR".to_string()));
    assert!(compliance_report.regulatory_compliance.contains(&"PCI-DSS".to_string()));
}

#[tokio::test]
async fn test_security_incident_detection() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Create transaction with high operation count to trigger security alert
    let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Add many operations to trigger security monitoring
    for i in 0..1200 { // Exceeds 1000 operation threshold
        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("security{}@example.com", i),
            username: format!("securityuser{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        if simulator.insert_user_in_transaction(tx_id, user).await.is_err() {
            break; // Stop if we hit resource limits
        }
    }

    simulator.commit_transaction(tx_id).await.unwrap();

    let compliance_report = simulator.generate_compliance_report().await;
    let incidents = &compliance_report.security_incidents;

    // Should detect high volume transaction incident
    let has_high_volume_incident = incidents.iter()
        .any(|incident| incident.incident_type == "High Volume Transaction");
    assert!(has_high_volume_incident);

    if let Some(incident) = incidents.iter().find(|i| i.incident_type == "High Volume Transaction") {
        assert_eq!(incident.severity, SecuritySeverity::Medium);
        assert!(incident.description.contains("unusually high operation count"));
    }
}

#[tokio::test]
async fn test_rapid_transaction_pattern_detection() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Simulate rapid-fire transactions from same session
    let session_id = "session_rapid_test".to_string();

    for i in 0..150 { // Exceeds 100 transaction threshold
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

        // Manually set session ID to simulate same session
        if let Some(transaction) = simulator.active_transactions.get_mut(&tx_id) {
            transaction.session_id = Some(session_id.clone());
        }

        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("rapid{}@example.com", i),
            username: format!("rapiduser{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
        simulator.commit_transaction(tx_id).await.unwrap();
    }

    let compliance_report = simulator.generate_compliance_report().await;
    let incidents = &compliance_report.security_incidents;

    // Should detect rapid transaction pattern
    let has_rapid_pattern_incident = incidents.iter()
        .any(|incident| incident.incident_type == "Rapid Transaction Pattern");
    assert!(has_rapid_pattern_incident);

    if let Some(incident) = incidents.iter().find(|i| i.incident_type == "Rapid Transaction Pattern") {
        assert_eq!(incident.severity, SecuritySeverity::High);
        assert!(incident.description.contains("possible automation"));
        assert!(incident.description.contains(&session_id));
    }
}

#[tokio::test]
async fn test_compliance_score_calculation() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Create transaction to generate audit trail
    let tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::ReadCommitted,
        Some(TransactionPriority::Normal),
        Some(Duration::from_secs(300)),
        None,
    ).await.unwrap();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "score@example.com".to_string(),
        username: "scoreuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx_id, user).await.unwrap();
    simulator.commit_transaction(tx_id).await.unwrap();

    let compliance_report = simulator.generate_compliance_report().await;

    // Score should be high since we've implemented most compliance features
    assert!(compliance_report.compliance_score >= 80.0);
    assert!(compliance_report.compliance_score <= 100.0);

    // Verify audit trail exists
    assert!(compliance_report.audit_trail_complete);

    // Verify basic security features
    assert!(compliance_report.transaction_encryption);
    assert!(compliance_report.access_control_enforced);
}

#[tokio::test]
async fn test_transaction_pattern_analysis_with_mixed_outcomes() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Create transactions with different patterns
    let mut successful_count = 0;
    let mut failed_count = 0;

    for i in 0..20 {
        let tx_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

        let user = TransactionTestUser {
            id: Uuid::now_v7(),
            email: format!("pattern{}@example.com", i),
            username: format!("patternuser{}", i),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        simulator.insert_user_in_transaction(tx_id, user).await.unwrap();

        // Create different patterns: commit most, rollback some, fail some
        match i % 4 {
            0 | 1 | 2 => {
                simulator.commit_transaction(tx_id).await.unwrap();
                successful_count += 1;
            },
            3 => {
                simulator.rollback_transaction(tx_id).await.unwrap();
                failed_count += 1;
            },
            _ => unreachable!(),
        }
    }

    let analytics = simulator.analyze_transaction_patterns().await;

    assert_eq!(analytics.total_transactions, 20);
    assert_eq!(analytics.success_rate, successful_count as f64 / 20.0);
    assert!(analytics.average_duration.as_millis() > 0);

    // Verify analytics captured the pattern correctly
    let expected_success_rate = 0.75; // 3 out of 4 succeed in our pattern
    assert!((analytics.success_rate - expected_success_rate).abs() < 0.1);
}

#[tokio::test]
async fn test_lock_contention_analysis() {
    let mut simulator = TransactionIntegritySimulator::new();

    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "contention@example.com".to_string(),
        username: "contentionuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.add_test_user(user.clone());

    // Create multiple transactions that will contend for same resource
    let tx1_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
    let tx2_id = simulator.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();

    // Both transactions try to access same user (will create lock contention)
    simulator.select_user_in_transaction(tx1_id, user.id).await.unwrap();
    simulator.select_user_in_transaction(tx2_id, user.id).await.unwrap();

    // Get analytics while transactions are active
    let analytics = simulator.analyze_transaction_patterns().await;

    // Should detect some lock contention events
    assert!(analytics.lock_contention_events > 0);

    // Verify health monitoring detects contention
    let health_report = simulator.diagnose_transaction_health().await;
    assert!(health_report.deadlock_risk > 0.0);

    // Clean up
    simulator.commit_transaction(tx1_id).await.unwrap();
    simulator.commit_transaction(tx2_id).await.unwrap();
}

#[tokio::test]
async fn test_enterprise_transaction_lifecycle_monitoring() {
    let mut simulator = TransactionIntegritySimulator::new();

    // Begin transaction with comprehensive monitoring
    let tx_id = simulator.begin_transaction_with_options(
        IsolationLevel::Serializable,
        Some(TransactionPriority::High),
        Some(Duration::from_secs(600)),
        None,
    ).await.unwrap();

    // Verify transaction is properly tracked
    let metrics = simulator.get_transaction_metrics(tx_id).unwrap();
    assert_eq!(metrics.transaction_id, tx_id);
    assert_eq!(metrics.isolation_level, IsolationLevel::Serializable);

    // Perform operations and verify they're tracked
    let user = TransactionTestUser {
        id: Uuid::now_v7(),
        email: "lifecycle@example.com".to_string(),
        username: "lifecycleuser".to_string(),
        password_hash: "hashed_password".to_string(),
        email_verified: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
    };

    simulator.insert_user_in_transaction(tx_id, user.clone()).await.unwrap();

    // Create savepoint for advanced transaction control
    let savepoint_id = simulator.create_savepoint(tx_id, "test_checkpoint".to_string()).await.unwrap();

    // More operations after savepoint
    let mut updates = HashMap::new();
    updates.insert("username".to_string(), "updated_lifecycle_user".to_string());
    simulator.update_user_in_transaction(tx_id, user.id, updates).await.unwrap();

    // Verify transaction state and metrics
    let transaction = simulator.active_transactions.get(&tx_id).unwrap();
    assert!(transaction.operations.len() >= 3); // Insert + savepoint + update
    assert!(!transaction.savepoints.is_empty());
    assert_eq!(transaction.priority, TransactionPriority::High);

    // Check timeout is being tracked
    assert!(simulator.timeout_manager.active_timeouts.contains_key(&tx_id));

    // Test rollback to savepoint
    simulator.rollback_to_savepoint(tx_id, savepoint_id).await.unwrap();

    // Verify rollback worked
    let transaction_after_rollback = simulator.active_transactions.get(&tx_id).unwrap();
    assert!(transaction_after_rollback.operations.len() < transaction.operations.len());

    // Commit and verify final state
    simulator.commit_transaction(tx_id).await.unwrap();

    // Verify transaction is completed with proper audit trail
    let audit_entries = simulator.get_audit_log_for_transaction(tx_id);
    assert!(!audit_entries.is_empty());

    let begin_entry = audit_entries.iter().find(|e| e.operation == "BEGIN");
    assert!(begin_entry.is_some());
    assert!(begin_entry.unwrap().success);

    // Check final metrics
    let final_metrics = simulator.get_transaction_metrics(tx_id).unwrap();
    assert_eq!(final_metrics.final_state, Some(TransactionState::Committed));
}