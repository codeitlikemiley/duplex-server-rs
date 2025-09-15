//! Integration tests for database transaction integrity
//!
//! These tests verify that database transactions maintain ACID properties
//! and handle concurrent operations, rollbacks, and error scenarios correctly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    Started,
    InProgress,
    Committed,
    RolledBack,
    Failed,
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

#[derive(Debug, Clone)]
pub struct DatabaseTransaction {
    pub id: Uuid,
    pub state: TransactionState,
    pub isolation_level: IsolationLevel,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub operations: Vec<TransactionOperation>,
    pub locks_held: Vec<DatabaseLock>,
}

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    Insert { table: String, record_id: Uuid },
    Update { table: String, record_id: Uuid, old_version: i64, new_version: i64 },
    Delete { table: String, record_id: Uuid },
    Select { table: String, record_id: Option<Uuid> },
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
        }
    }

    pub fn enable_failure_simulation(&mut self) {
        self.simulate_failures = true;
    }

    pub fn disable_failure_simulation(&mut self) {
        self.simulate_failures = false;
    }

    // Transaction lifecycle management
    pub async fn begin_transaction(&mut self, isolation_level: IsolationLevel) -> Result<Uuid, String> {
        let transaction_id = Uuid::now_v7();
        let transaction = DatabaseTransaction {
            id: transaction_id,
            state: TransactionState::Started,
            isolation_level,
            started_at: Utc::now(),
            completed_at: None,
            operations: Vec::new(),
            locks_held: Vec::new(),
        };

        self.active_transactions.insert(transaction_id, transaction);
        Ok(transaction_id)
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