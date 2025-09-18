//! Integration tests for session management and logout operations
//! Enhanced with enterprise features for concurrent session management
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, Semaphore};
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
    pub display_name: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub failed_login_attempts: u32,
    pub account_locked_until: Option<DateTime<Utc>>,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    pub recovery_email: Option<String>,
    pub recovery_email_verified: bool,
    pub phone_number: Option<String>,
    pub phone_verified: bool,
    pub profile_picture_url: Option<String>,
    pub bio: Option<String>,
    pub preferences: serde_json::Value,
    pub metadata: serde_json::Value,
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
    InternalServerError {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub is_active: bool,
    pub device_name: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

#[derive(Debug, Clone)]
pub struct SessionActivity {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub ip_address: String,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub last_seen: DateTime<Utc>,
    pub trusted: bool,
}

// Enhanced structures for concurrent session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrentSessionPolicy {
    pub max_concurrent_sessions: usize,
    pub force_single_device: bool,
    pub allow_concurrent_same_ip: bool,
    pub session_collision_strategy: SessionCollisionStrategy,
    pub concurrent_login_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionCollisionStrategy {
    RejectNew,
    RevokeOldest,
    RevokeAll,
    AllowOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSyncEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub event_type: SessionEventType,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEventType {
    Created,
    Validated,
    Extended,
    Revoked,
    Expired,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLock {
    pub session_id: Uuid,
    pub locked_by: String,
    pub locked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrentSessionMetrics {
    pub total_concurrent_attempts: u64,
    pub successful_concurrent_sessions: u64,
    pub failed_concurrent_sessions: u64,
    pub collision_resolutions: u64,
    pub average_session_duration_minutes: f64,
    pub peak_concurrent_sessions: usize,
    pub session_conflicts_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReplication {
    pub id: Uuid,
    pub session_id: Uuid,
    pub node_id: String,
    pub replicated_at: DateTime<Utc>,
    pub status: ReplicationStatus,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationStatus {
    Pending,
    Synchronized,
    Failed,
    Inconsistent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerSession {
    pub session_id: Uuid,
    pub sticky_node: String,
    pub backup_nodes: Vec<String>,
    pub last_routing_update: DateTime<Utc>,
    pub traffic_weight: f32,
}

pub struct SessionManagementSimulator {
    users: Vec<User>,
    sessions: HashMap<Uuid, Session>,
    refresh_tokens: HashMap<String, RefreshToken>,
    session_activities: HashMap<Uuid, Vec<SessionActivity>>,
    trusted_devices: HashMap<Uuid, Vec<DeviceInfo>>,
    max_sessions_per_user: usize,
    session_timeout: Duration,
    refresh_token_lifetime: Duration,
    remember_me_duration: Duration,
    // Enhanced concurrent session management
    concurrent_session_policy: ConcurrentSessionPolicy,
    session_sync_events: Vec<SessionSyncEvent>,
    session_locks: HashMap<Uuid, SessionLock>,
    concurrent_metrics: ConcurrentSessionMetrics,
    session_replications: HashMap<Uuid, Vec<SessionReplication>>,
    load_balancer_sessions: HashMap<Uuid, LoadBalancerSession>,
    active_concurrent_operations: Arc<Semaphore>,
    session_conflict_queue: VecDeque<(Uuid, String)>, // user_id, conflict_reason
    distributed_session_cache: Arc<RwLock<HashMap<String, String>>>,
    node_health_status: HashMap<String, bool>,
    session_failover_history: Vec<(Uuid, String, String)>, // session_id, from_node, to_node
}

impl SessionManagementSimulator {
    pub fn new() -> Self {
        Self {
            users: vec![],
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            session_activities: HashMap::new(),
            trusted_devices: HashMap::new(),
            max_sessions_per_user: 5,
            session_timeout: Duration::hours(2),
            refresh_token_lifetime: Duration::days(30),
            remember_me_duration: Duration::days(90),
            // Initialize enhanced features
            concurrent_session_policy: ConcurrentSessionPolicy {
                max_concurrent_sessions: 10,
                force_single_device: false,
                allow_concurrent_same_ip: true,
                session_collision_strategy: SessionCollisionStrategy::RevokeOldest,
                concurrent_login_delay_ms: 100,
            },
            session_sync_events: Vec::new(),
            session_locks: HashMap::new(),
            concurrent_metrics: ConcurrentSessionMetrics {
                total_concurrent_attempts: 0,
                successful_concurrent_sessions: 0,
                failed_concurrent_sessions: 0,
                collision_resolutions: 0,
                average_session_duration_minutes: 0.0,
                peak_concurrent_sessions: 0,
                session_conflicts_per_hour: 0.0,
            },
            session_replications: HashMap::new(),
            load_balancer_sessions: HashMap::new(),
            active_concurrent_operations: Arc::new(Semaphore::new(100)),
            session_conflict_queue: VecDeque::new(),
            distributed_session_cache: Arc::new(RwLock::new(HashMap::new())),
            node_health_status: HashMap::new(),
            session_failover_history: Vec::new(),
        }
    }

    pub async fn create_test_user(&mut self) -> Uuid {
        let user = User {
            id: Uuid::now_v7(),
            email: format!("user{}@example.com", Uuid::now_v7()),
            username: format!("user_{}", Uuid::now_v7()),
            display_name: "Test User".to_string(),
            password_hash: "hashed_password".to_string(),
            status: UserStatus::Active,
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            failed_login_attempts: 0,
            account_locked_until: None,
            password_changed_at: Some(Utc::now()),
            two_factor_enabled: false,
            two_factor_secret: None,
            recovery_email: None,
            recovery_email_verified: false,
            phone_number: None,
            phone_verified: false,
            profile_picture_url: None,
            bio: None,
            preferences: serde_json::json!({}),
            metadata: serde_json::json!({}),
        };
        let user_id = user.id;
        self.users.push(user);
        user_id
    }

    pub async fn create_session(
        &mut self,
        user_id: Uuid,
        ip_address: String,
        user_agent: String,
        remember_me: bool,
    ) -> Result<(String, String), AppError> {
        // Check if user exists and is active
        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if user.status != UserStatus::Active {
            return Err(AppError::Unauthorized {
                message: "User account is not active".to_string(),
            });
        }

        // Check max sessions limit
        let active_sessions = self.sessions.values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .count();

        if active_sessions >= self.max_sessions_per_user {
            // Revoke oldest session
            if let Some(oldest) = self.sessions.values_mut()
                .filter(|s| s.user_id == user_id && s.is_active)
                .min_by_key(|s| s.created_at)
            {
                oldest.is_active = false;
            }
        }

        // Create new session
        let session = Session {
            id: Uuid::now_v7(),
            user_id,
            token: format!("session_{}", Uuid::now_v7()),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            expires_at: if remember_me {
                Utc::now() + self.remember_me_duration
            } else {
                Utc::now() + self.session_timeout
            },
            ip_address: ip_address.clone(),
            user_agent,
            is_active: true,
            device_name: None,
            location: None,
        };

        // Create refresh token
        let refresh_token = RefreshToken {
            token: format!("refresh_{}", Uuid::now_v7()),
            session_id: session.id,
            expires_at: Utc::now() + self.refresh_token_lifetime,
            used: false,
        };

        // Log activity
        self.session_activities.entry(session.id)
            .or_insert_with(Vec::new)
            .push(SessionActivity {
                timestamp: Utc::now(),
                action: "session_created".to_string(),
                ip_address,
            });

        let session_token = session.token.clone();
        let refresh_token_str = refresh_token.token.clone();

        self.sessions.insert(session.id, session);
        self.refresh_tokens.insert(refresh_token_str.clone(), refresh_token);

        Ok((session_token, refresh_token_str))
    }

    pub async fn validate_session(&mut self, token: &str) -> Result<Uuid, AppError> {
        let session = self.sessions.values_mut()
            .find(|s| s.token == token)
            .ok_or_else(|| AppError::Unauthorized {
                message: "Invalid session token".to_string(),
            })?;

        if !session.is_active {
            return Err(AppError::Unauthorized {
                message: "Session has been revoked".to_string(),
            });
        }

        if Utc::now() > session.expires_at {
            session.is_active = false;
            return Err(AppError::Unauthorized {
                message: "Session has expired".to_string(),
            });
        }

        // Update last activity
        session.last_activity = Utc::now();

        // Extend session if within timeout window
        if session.expires_at - Utc::now() < Duration::minutes(30) {
            session.expires_at = Utc::now() + self.session_timeout;
        }

        Ok(session.user_id)
    }

    pub async fn refresh_session(&mut self, refresh_token: &str) -> Result<(String, String), AppError> {
        let token_data = self.refresh_tokens.get_mut(refresh_token)
            .ok_or_else(|| AppError::Unauthorized {
                message: "Invalid refresh token".to_string(),
            })?;

        if token_data.used {
            // Potential token reuse attack - revoke all sessions
            let session_id = token_data.session_id;
            if let Some(session) = self.sessions.get_mut(&session_id) {
                let user_id = session.user_id;
                // Revoke all user sessions
                for session in self.sessions.values_mut() {
                    if session.user_id == user_id {
                        session.is_active = false;
                    }
                }
            }
            return Err(AppError::Unauthorized {
                message: "Refresh token has already been used".to_string(),
            });
        }

        if Utc::now() > token_data.expires_at {
            return Err(AppError::Unauthorized {
                message: "Refresh token has expired".to_string(),
            });
        }

        // Mark token as used
        token_data.used = true;
        let session_id = token_data.session_id;

        // Get and update session
        let session = self.sessions.get_mut(&session_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "session".to_string(),
                id: Some(session_id.to_string()),
            })?;

        if !session.is_active {
            return Err(AppError::Unauthorized {
                message: "Session has been revoked".to_string(),
            });
        }

        // Create new tokens
        session.token = format!("session_{}", Uuid::now_v7());
        session.expires_at = Utc::now() + self.session_timeout;
        session.last_activity = Utc::now();

        let new_refresh_token = RefreshToken {
            token: format!("refresh_{}", Uuid::now_v7()),
            session_id,
            expires_at: Utc::now() + self.refresh_token_lifetime,
            used: false,
        };

        let session_token = session.token.clone();
        let refresh_token_str = new_refresh_token.token.clone();

        self.refresh_tokens.insert(refresh_token_str.clone(), new_refresh_token);

        Ok((session_token, refresh_token_str))
    }

    pub async fn logout(&mut self, token: &str) -> Result<(), AppError> {
        let session = self.sessions.values_mut()
            .find(|s| s.token == token)
            .ok_or_else(|| AppError::NotFound {
                resource: "session".to_string(),
                id: None,
            })?;

        if !session.is_active {
            return Ok(()); // Already logged out
        }

        session.is_active = false;

        // Log activity
        let session_id = session.id;
        let ip_address = session.ip_address.clone();
        self.session_activities.entry(session_id)
            .or_insert_with(Vec::new)
            .push(SessionActivity {
                timestamp: Utc::now(),
                action: "logout".to_string(),
                ip_address,
            });

        Ok(())
    }

    pub async fn logout_all_sessions(&mut self, user_id: Uuid) -> Result<usize, AppError> {
        let mut count = 0;
        for session in self.sessions.values_mut() {
            if session.user_id == user_id && session.is_active {
                session.is_active = false;
                count += 1;

                // Log activity
                self.session_activities.entry(session.id)
                    .or_insert_with(Vec::new)
                    .push(SessionActivity {
                        timestamp: Utc::now(),
                        action: "logout_all".to_string(),
                        ip_address: session.ip_address.clone(),
                    });
            }
        }
        Ok(count)
    }

    pub async fn logout_other_sessions(&mut self, token: &str) -> Result<usize, AppError> {
        let current_session = self.sessions.values()
            .find(|s| s.token == token)
            .ok_or_else(|| AppError::NotFound {
                resource: "session".to_string(),
                id: None,
            })?;

        let user_id = current_session.user_id;
        let current_session_id = current_session.id;

        let mut count = 0;
        for session in self.sessions.values_mut() {
            if session.user_id == user_id && session.id != current_session_id && session.is_active {
                session.is_active = false;
                count += 1;

                // Log activity
                self.session_activities.entry(session.id)
                    .or_insert_with(Vec::new)
                    .push(SessionActivity {
                        timestamp: Utc::now(),
                        action: "logout_other".to_string(),
                        ip_address: session.ip_address.clone(),
                    });
            }
        }
        Ok(count)
    }

    pub async fn get_active_sessions(&self, user_id: Uuid) -> Vec<Session> {
        self.sessions.values()
            .filter(|s| s.user_id == user_id && s.is_active)
            .cloned()
            .collect()
    }

    pub async fn revoke_session(&mut self, session_id: Uuid, admin_user_id: Uuid) -> Result<(), AppError> {
        let session = self.sessions.get_mut(&session_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "session".to_string(),
                id: Some(session_id.to_string()),
            })?;

        // Check if admin or self
        if session.user_id != admin_user_id {
            // In real implementation, check admin permissions
            // For testing, we'll allow it
        }

        session.is_active = false;

        // Log activity
        self.session_activities.entry(session_id)
            .or_insert_with(Vec::new)
            .push(SessionActivity {
                timestamp: Utc::now(),
                action: "admin_revoke".to_string(),
                ip_address: "admin".to_string(),
            });

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&mut self) -> usize {
        let now = Utc::now();
        let mut count = 0;

        for session in self.sessions.values_mut() {
            if session.is_active && now > session.expires_at {
                session.is_active = false;
                count += 1;
            }
        }

        // Clean up old refresh tokens
        self.refresh_tokens.retain(|_, token| {
            now <= token.expires_at
        });

        count
    }

    pub async fn add_trusted_device(&mut self, user_id: Uuid, device_id: String, device_name: String) -> Result<(), AppError> {
        let device = DeviceInfo {
            device_id: device_id.clone(),
            device_name,
            last_seen: Utc::now(),
            trusted: true,
        };

        self.trusted_devices.entry(user_id)
            .or_insert_with(Vec::new)
            .push(device);

        Ok(())
    }

    pub async fn is_trusted_device(&self, user_id: Uuid, device_id: &str) -> bool {
        self.trusted_devices.get(&user_id)
            .map(|devices| devices.iter().any(|d| d.device_id == device_id && d.trusted))
            .unwrap_or(false)
    }

    pub async fn remove_trusted_device(&mut self, user_id: Uuid, device_id: &str) -> Result<(), AppError> {
        if let Some(devices) = self.trusted_devices.get_mut(&user_id) {
            devices.retain(|d| d.device_id != device_id);
            Ok(())
        } else {
            Err(AppError::NotFound {
                resource: "device".to_string(),
                id: Some(device_id.to_string()),
            })
        }
    }

    // Enhanced concurrent session management methods
    pub async fn create_concurrent_session(
        &mut self,
        user_id: Uuid,
        ip_address: String,
        user_agent: String,
        device_id: String,
        remember_me: bool,
    ) -> Result<(String, String), AppError> {
        // Acquire semaphore permit for concurrent operation
        let _permit = self.active_concurrent_operations.try_acquire()
            .map_err(|_| AppError::InternalServerError {
                message: "Too many concurrent operations".to_string(),
            })?;

        self.concurrent_metrics.total_concurrent_attempts += 1;

        // Check for device conflicts if single device policy is enforced
        if self.concurrent_session_policy.force_single_device {
            let existing_device_sessions = self.get_sessions_by_device(&device_id).await;
            if !existing_device_sessions.is_empty() {
                self.handle_session_collision(user_id, "device_conflict").await?;
            }
        }

        // Check for IP-based conflicts
        if !self.concurrent_session_policy.allow_concurrent_same_ip {
            let existing_ip_sessions = self.get_sessions_by_ip(&ip_address).await;
            if existing_ip_sessions.iter().any(|s| s.user_id == user_id) {
                self.handle_session_collision(user_id, "ip_conflict").await?;
            }
        }

        // Simulate concurrent login delay
        if self.concurrent_session_policy.concurrent_login_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.concurrent_session_policy.concurrent_login_delay_ms
            )).await;
        }

        // Create session with distributed locking
        let session_result = self.create_session_with_lock(
            user_id,
            ip_address.clone(),
            user_agent,
            remember_me
        ).await;

        match session_result {
            Ok((session_token, refresh_token)) => {
                self.concurrent_metrics.successful_concurrent_sessions += 1;

                // Update peak concurrent sessions
                let current_active = self.get_active_sessions(user_id).await.len();
                if current_active > self.concurrent_metrics.peak_concurrent_sessions {
                    self.concurrent_metrics.peak_concurrent_sessions = current_active;
                }

                // Log sync event
                if let Some(session) = self.sessions.values().find(|s| s.token == session_token) {
                    self.log_session_sync_event(
                        session.id,
                        SessionEventType::Created,
                        serde_json::json!({"device_id": device_id, "ip": ip_address})
                    ).await;
                }

                Ok((session_token, refresh_token))
            },
            Err(e) => {
                self.concurrent_metrics.failed_concurrent_sessions += 1;
                Err(e)
            }
        }
    }

    async fn create_session_with_lock(
        &mut self,
        user_id: Uuid,
        ip_address: String,
        user_agent: String,
        remember_me: bool,
    ) -> Result<(String, String), AppError> {
        // Create distributed lock
        let lock_id = format!("session_create_{}", user_id);
        let lock = SessionLock {
            session_id: Uuid::now_v7(),
            locked_by: lock_id.clone(),
            locked_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(30),
            reason: "concurrent_session_creation".to_string(),
        };

        // Check if already locked
        if let Some(existing_lock) = self.session_locks.get(&user_id) {
            if existing_lock.expires_at > Utc::now() {
                return Err(AppError::BadRequest {
                    message: "Session creation already in progress".to_string(),
                });
            }
        }

        self.session_locks.insert(user_id, lock);

        // Create session using existing logic
        let result = self.create_session(user_id, ip_address, user_agent, remember_me).await;

        // Release lock
        self.session_locks.remove(&user_id);

        result
    }

    async fn handle_session_collision(&mut self, user_id: Uuid, reason: &str) -> Result<(), AppError> {
        self.concurrent_metrics.collision_resolutions += 1;
        self.session_conflict_queue.push_back((user_id, reason.to_string()));

        match self.concurrent_session_policy.session_collision_strategy {
            SessionCollisionStrategy::RejectNew => {
                Err(AppError::BadRequest {
                    message: format!("Session collision detected: {}", reason),
                })
            },
            SessionCollisionStrategy::RevokeOldest => {
                self.revoke_oldest_session(user_id).await;
                Ok(())
            },
            SessionCollisionStrategy::RevokeAll => {
                self.logout_all_sessions(user_id).await?;
                Ok(())
            },
            SessionCollisionStrategy::AllowOverride => {
                // Log the conflict but allow creation
                self.log_session_sync_event(
                    Uuid::new_v4(),
                    SessionEventType::Conflicted,
                    serde_json::json!({"reason": reason, "user_id": user_id})
                ).await;
                Ok(())
            }
        }
    }

    async fn revoke_oldest_session(&mut self, user_id: Uuid) {
        if let Some(oldest_session) = self.sessions.values_mut()
            .filter(|s| s.user_id == user_id && s.is_active)
            .min_by_key(|s| s.created_at)
        {
            oldest_session.is_active = false;

            self.log_session_sync_event(
                oldest_session.id,
                SessionEventType::Revoked,
                serde_json::json!({"reason": "collision_resolution"})
            ).await;
        }
    }

    async fn get_sessions_by_device(&self, device_id: &str) -> Vec<&Session> {
        // In a real implementation, device_id would be stored in session
        // For testing, we'll simulate this
        self.sessions.values()
            .filter(|s| s.is_active && s.user_agent.contains(device_id))
            .collect()
    }

    async fn get_sessions_by_ip(&self, ip_address: &str) -> Vec<&Session> {
        self.sessions.values()
            .filter(|s| s.is_active && s.ip_address == ip_address)
            .collect()
    }

    async fn log_session_sync_event(
        &mut self,
        session_id: Uuid,
        event_type: SessionEventType,
        data: serde_json::Value,
    ) {
        let event = SessionSyncEvent {
            id: Uuid::now_v7(),
            session_id,
            event_type,
            timestamp: Utc::now(),
            data,
        };
        self.session_sync_events.push(event);
    }

    pub async fn simulate_session_replication(
        &mut self,
        session_id: Uuid,
        nodes: Vec<String>,
    ) -> Result<Vec<Uuid>, AppError> {
        let session = self.sessions.get(&session_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "session".to_string(),
                id: Some(session_id.to_string()),
            })?;

        let mut replication_ids = Vec::new();

        for node in nodes {
            let replication = SessionReplication {
                id: Uuid::now_v7(),
                session_id,
                node_id: node.clone(),
                replicated_at: Utc::now(),
                status: ReplicationStatus::Synchronized,
                checksum: format!("checksum_{}", session.token),
            };

            replication_ids.push(replication.id);
            self.session_replications.entry(session_id)
                .or_insert_with(Vec::new)
                .push(replication);

            // Simulate load balancer session
            let lb_session = LoadBalancerSession {
                session_id,
                sticky_node: node.clone(),
                backup_nodes: nodes.iter().filter(|n| **n != node).cloned().collect(),
                last_routing_update: Utc::now(),
                traffic_weight: 1.0 / nodes.len() as f32,
            };

            self.load_balancer_sessions.insert(session_id, lb_session);
        }

        Ok(replication_ids)
    }

    pub async fn simulate_node_failure(&mut self, failed_node: &str) -> Result<Vec<Uuid>, AppError> {
        self.node_health_status.insert(failed_node.to_string(), false);
        let mut failed_over_sessions = Vec::new();

        // Find sessions on failed node and trigger failover
        for (session_id, replications) in &mut self.session_replications {
            for replication in replications {
                if replication.node_id == failed_node && replication.status == ReplicationStatus::Synchronized {
                    replication.status = ReplicationStatus::Failed;

                    // Find backup node
                    if let Some(lb_session) = self.load_balancer_sessions.get_mut(session_id) {
                        if let Some(backup_node) = lb_session.backup_nodes.first() {
                            lb_session.sticky_node = backup_node.clone();
                            lb_session.last_routing_update = Utc::now();

                            self.session_failover_history.push((
                                *session_id,
                                failed_node.to_string(),
                                backup_node.clone()
                            ));

                            failed_over_sessions.push(*session_id);
                        }
                    }
                }
            }
        }

        Ok(failed_over_sessions)
    }

    pub async fn validate_session_consistency(&self) -> Vec<(Uuid, String)> {
        let mut inconsistencies = Vec::new();

        for (session_id, replications) in &self.session_replications {
            let checksums: std::collections::HashSet<&String> = replications
                .iter()
                .filter(|r| r.status == ReplicationStatus::Synchronized)
                .map(|r| &r.checksum)
                .collect();

            if checksums.len() > 1 {
                inconsistencies.push((*session_id, "checksum_mismatch".to_string()));
            }

            // Check if session exists locally but not in distributed cache
            if self.sessions.contains_key(session_id) {
                let cache = self.distributed_session_cache.blocking_read();
                if !cache.contains_key(&session_id.to_string()) {
                    inconsistencies.push((*session_id, "missing_from_cache".to_string()));
                }
            }
        }

        inconsistencies
    }

    pub async fn get_concurrent_session_metrics(&self) -> ConcurrentSessionMetrics {
        let mut metrics = self.concurrent_metrics.clone();

        // Calculate average session duration
        let total_duration: i64 = self.sessions.values()
            .filter(|s| !s.is_active)
            .map(|s| (s.last_activity - s.created_at).num_minutes())
            .sum();

        let completed_sessions = self.sessions.values()
            .filter(|s| !s.is_active)
            .count();

        if completed_sessions > 0 {
            metrics.average_session_duration_minutes = total_duration as f64 / completed_sessions as f64;
        }

        // Calculate conflicts per hour
        let conflicts_last_hour = self.session_sync_events.iter()
            .filter(|e| {
                matches!(e.event_type, SessionEventType::Conflicted) &&
                e.timestamp > Utc::now() - Duration::hours(1)
            })
            .count();

        metrics.session_conflicts_per_hour = conflicts_last_hour as f64;

        metrics
    }

    pub async fn simulate_concurrent_logins(
        &mut self,
        user_id: Uuid,
        concurrent_count: usize,
    ) -> Result<Vec<Result<(String, String), AppError>>, AppError> {
        let mut results = Vec::new();

        // Create concurrent login attempts
        for i in 0..concurrent_count {
            let ip = format!("192.168.1.{}", i + 1);
            let user_agent = format!("Browser_{}", i);
            let device_id = format!("device_{}", i);

            // Simulate concurrent operations by creating futures
            let result = self.create_concurrent_session(
                user_id,
                ip,
                user_agent,
                device_id,
                false
            ).await;

            results.push(result);
        }

        Ok(results)
    }

    pub async fn test_session_scalability(&mut self, target_sessions: usize) -> Result<Duration, AppError> {
        let start_time = std::time::Instant::now();
        let mut created_sessions = 0;

        while created_sessions < target_sessions {
            let user_id = self.create_test_user().await;

            match self.create_concurrent_session(
                user_id,
                format!("192.168.{}.{}", created_sessions / 256, created_sessions % 256),
                "LoadTest/1.0".to_string(),
                format!("device_{}", created_sessions),
                false
            ).await {
                Ok(_) => created_sessions += 1,
                Err(_) => {
                    // Continue with next attempt
                }
            }

            // Prevent infinite loops
            if start_time.elapsed() > std::time::Duration::from_secs(30) {
                break;
            }
        }

        Ok(Duration::milliseconds(start_time.elapsed().as_millis() as i64))
    }

    pub async fn cleanup_session_resources(&mut self) -> usize {
        let mut cleaned = 0;

        // Clean up expired locks
        let now = Utc::now();
        self.session_locks.retain(|_, lock| {
            if now > lock.expires_at {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        // Clean up old sync events (keep last 1000)
        if self.session_sync_events.len() > 1000 {
            let excess = self.session_sync_events.len() - 1000;
            self.session_sync_events.drain(0..excess);
            cleaned += excess;
        }

        // Clean up session conflict queue
        while self.session_conflict_queue.len() > 100 {
            self.session_conflict_queue.pop_front();
            cleaned += 1;
        }

        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let result = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await;

        assert!(result.is_ok());
        let (session_token, refresh_token) = result.unwrap();
        assert!(!session_token.is_empty());
        assert!(!refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_validate_session() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (session_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        let result = simulator.validate_session(&session_token).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), user_id);
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (session_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        // Manually expire the session
        if let Some(session) = simulator.sessions.values_mut().find(|s| s.token == session_token) {
            session.expires_at = Utc::now() - Duration::hours(1);
        }

        let result = simulator.validate_session(&session_token).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Unauthorized { .. }));
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (_, refresh_token) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        let result = simulator.refresh_session(&refresh_token).await;
        assert!(result.is_ok());

        let (new_session_token, new_refresh_token) = result.unwrap();
        assert!(!new_session_token.is_empty());
        assert!(!new_refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_reuse_prevention() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (_, refresh_token) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        // Use refresh token once
        simulator.refresh_session(&refresh_token).await.unwrap();

        // Try to reuse the same refresh token
        let result = simulator.refresh_session(&refresh_token).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Unauthorized { .. }));

        // Verify all user sessions are revoked (security measure)
        let active_sessions = simulator.get_active_sessions(user_id).await;
        assert_eq!(active_sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_logout() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (session_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        // Logout
        let result = simulator.logout(&session_token).await;
        assert!(result.is_ok());

        // Try to use the session after logout
        let validation_result = simulator.validate_session(&session_token).await;
        assert!(validation_result.is_err());
    }

    #[tokio::test]
    async fn test_logout_all_sessions() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create multiple sessions
        for i in 0..3 {
            simulator.create_session(
                user_id,
                format!("192.168.1.{}", i),
                "Mozilla/5.0".to_string(),
                false
            ).await.unwrap();
        }

        let active_before = simulator.get_active_sessions(user_id).await;
        assert_eq!(active_before.len(), 3);

        // Logout all sessions
        let count = simulator.logout_all_sessions(user_id).await.unwrap();
        assert_eq!(count, 3);

        let active_after = simulator.get_active_sessions(user_id).await;
        assert_eq!(active_after.len(), 0);
    }

    #[tokio::test]
    async fn test_logout_other_sessions() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create current session
        let (current_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        // Create other sessions
        for i in 2..4 {
            simulator.create_session(
                user_id,
                format!("192.168.1.{}", i),
                "Mozilla/5.0".to_string(),
                false
            ).await.unwrap();
        }

        let active_before = simulator.get_active_sessions(user_id).await;
        assert_eq!(active_before.len(), 3);

        // Logout other sessions
        let count = simulator.logout_other_sessions(&current_token).await.unwrap();
        assert_eq!(count, 2);

        let active_after = simulator.get_active_sessions(user_id).await;
        assert_eq!(active_after.len(), 1);

        // Current session should still be valid
        let validation = simulator.validate_session(&current_token).await;
        assert!(validation.is_ok());
    }

    #[tokio::test]
    async fn test_max_sessions_limit() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create max sessions
        let mut tokens = vec![];
        for i in 0..5 {
            let (token, _) = simulator.create_session(
                user_id,
                format!("192.168.1.{}", i),
                "Mozilla/5.0".to_string(),
                false
            ).await.unwrap();
            tokens.push(token);
        }

        // Create one more session (should revoke oldest)
        simulator.create_session(
            user_id,
            "192.168.1.100".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        // First session should be revoked
        let validation = simulator.validate_session(&tokens[0]).await;
        assert!(validation.is_err());

        // Other sessions should still be valid
        for token in &tokens[1..] {
            let validation = simulator.validate_session(token).await;
            assert!(validation.is_ok());
        }
    }

    #[tokio::test]
    async fn test_remember_me_duration() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create session with remember_me
        let (session_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            true // remember_me
        ).await.unwrap();

        // Check that session has extended expiration
        let session = simulator.sessions.values()
            .find(|s| s.token == session_token)
            .unwrap();

        let expected_expiry = Utc::now() + Duration::days(89); // Close to 90 days
        assert!(session.expires_at > expected_expiry);
    }

    #[tokio::test]
    async fn test_trusted_devices() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Add trusted device
        simulator.add_trusted_device(
            user_id,
            "device123".to_string(),
            "iPhone 12".to_string()
        ).await.unwrap();

        // Check if device is trusted
        assert!(simulator.is_trusted_device(user_id, "device123").await);
        assert!(!simulator.is_trusted_device(user_id, "unknown_device").await);

        // Remove trusted device
        simulator.remove_trusted_device(user_id, "device123").await.unwrap();
        assert!(!simulator.is_trusted_device(user_id, "device123").await);
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create sessions
        let mut session_ids = vec![];
        for i in 0..3 {
            let (token, _) = simulator.create_session(
                user_id,
                format!("192.168.1.{}", i),
                "Mozilla/5.0".to_string(),
                false
            ).await.unwrap();

            if let Some(session) = simulator.sessions.values().find(|s| s.token == token) {
                session_ids.push(session.id);
            }
        }

        // Manually expire some sessions
        for (i, session) in simulator.sessions.values_mut().enumerate() {
            if i < 2 {
                session.expires_at = Utc::now() - Duration::hours(1);
            }
        }

        // Run cleanup
        let cleaned = simulator.cleanup_expired_sessions().await;
        assert_eq!(cleaned, 2);

        // Check active sessions
        let active = simulator.get_active_sessions(user_id).await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_admin_session_revocation() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let (session_token, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string(),
            false
        ).await.unwrap();

        let session_id = simulator.sessions.values()
            .find(|s| s.token == session_token)
            .unwrap()
            .id;

        // Admin revokes user session
        simulator.revoke_session(session_id, admin_id).await.unwrap();

        // Session should be invalid
        let validation = simulator.validate_session(&session_token).await;
        assert!(validation.is_err());
    }

    // Enhanced concurrent session management tests
    #[tokio::test]
    async fn test_concurrent_session_creation() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Test concurrent session creation
        let result = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser_1".to_string(),
            "device_1".to_string(),
            false
        ).await;

        assert!(result.is_ok());
        let (session_token, refresh_token) = result.unwrap();
        assert!(!session_token.is_empty());
        assert!(!refresh_token.is_empty());

        // Verify metrics were updated
        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.total_concurrent_attempts, 1);
        assert_eq!(metrics.successful_concurrent_sessions, 1);
    }

    #[tokio::test]
    async fn test_session_collision_reject_new() {
        let mut simulator = SessionManagementSimulator::new();
        simulator.concurrent_session_policy.session_collision_strategy = SessionCollisionStrategy::RejectNew;
        simulator.concurrent_session_policy.force_single_device = true;

        let user_id = simulator.create_test_user().await;

        // Create first session
        let _result1 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await.unwrap();

        // Try to create second session on same device (should be rejected)
        let result2 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.2".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await;

        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("Session collision detected"));

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.collision_resolutions, 1);
    }

    #[tokio::test]
    async fn test_session_collision_revoke_oldest() {
        let mut simulator = SessionManagementSimulator::new();
        simulator.concurrent_session_policy.session_collision_strategy = SessionCollisionStrategy::RevokeOldest;
        simulator.concurrent_session_policy.force_single_device = true;

        let user_id = simulator.create_test_user().await;

        // Create first session
        let (session_token1, _) = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await.unwrap();

        // Create second session on same device (should revoke first)
        let result2 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.2".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await;

        assert!(result2.is_ok());

        // First session should be revoked
        let validation = simulator.validate_session(&session_token1).await;
        assert!(validation.is_err());

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.collision_resolutions, 1);
    }

    #[tokio::test]
    async fn test_session_distributed_locking() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create a session to establish a lock
        let lock = SessionLock {
            session_id: Uuid::now_v7(),
            locked_by: format!("session_create_{}", user_id),
            locked_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(30),
            reason: "concurrent_session_creation".to_string(),
        };

        simulator.session_locks.insert(user_id, lock);

        // Try to create session while locked (should fail)
        let result = simulator.create_session_with_lock(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            false
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Session creation already in progress"));
    }

    #[tokio::test]
    async fn test_concurrent_login_attempts() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Simulate multiple concurrent login attempts
        let results = simulator.simulate_concurrent_logins(user_id, 5).await.unwrap();

        assert_eq!(results.len(), 5);

        // Count successful and failed attempts
        let successful = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();

        assert!(successful > 0); // At least some should succeed
        println!("Concurrent logins: {} successful, {} failed", successful, failed);

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.total_concurrent_attempts, 5);
        assert!(metrics.peak_concurrent_sessions > 0);
    }

    #[tokio::test]
    async fn test_session_replication() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (_, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            false
        ).await.unwrap();

        let session_id = simulator.sessions.values().next().unwrap().id;
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];

        // Test session replication
        let replication_ids = simulator.simulate_session_replication(session_id, nodes.clone()).await.unwrap();

        assert_eq!(replication_ids.len(), 3);

        // Verify replications were created
        let replications = simulator.session_replications.get(&session_id).unwrap();
        assert_eq!(replications.len(), 3);

        for replication in replications {
            assert!(nodes.contains(&replication.node_id));
            assert_eq!(replication.status, ReplicationStatus::Synchronized);
        }

        // Verify load balancer session was created
        let lb_session = simulator.load_balancer_sessions.get(&session_id).unwrap();
        assert_eq!(lb_session.backup_nodes.len(), 2); // All nodes except sticky node
    }

    #[tokio::test]
    async fn test_node_failure_and_failover() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (_, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            false
        ).await.unwrap();

        let session_id = simulator.sessions.values().next().unwrap().id;
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];

        // Setup replication
        simulator.simulate_session_replication(session_id, nodes).await.unwrap();

        // Simulate node failure
        let failed_over_sessions = simulator.simulate_node_failure("node1").await.unwrap();

        assert!(!failed_over_sessions.is_empty());
        assert!(failed_over_sessions.contains(&session_id));

        // Verify node is marked as failed
        assert_eq!(simulator.node_health_status.get("node1"), Some(&false));

        // Verify failover history was recorded
        let failover_entry = simulator.session_failover_history.iter()
            .find(|(sid, from, _)| *sid == session_id && from == "node1");
        assert!(failover_entry.is_some());

        // Verify load balancer was updated
        let lb_session = simulator.load_balancer_sessions.get(&session_id).unwrap();
        assert_ne!(lb_session.sticky_node, "node1"); // Should have failed over
    }

    #[tokio::test]
    async fn test_session_consistency_validation() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        let (_, _) = simulator.create_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            false
        ).await.unwrap();

        let session_id = simulator.sessions.values().next().unwrap().id;

        // Create replications with inconsistent checksums
        let replication1 = SessionReplication {
            id: Uuid::now_v7(),
            session_id,
            node_id: "node1".to_string(),
            replicated_at: Utc::now(),
            status: ReplicationStatus::Synchronized,
            checksum: "checksum_1".to_string(),
        };

        let replication2 = SessionReplication {
            id: Uuid::now_v7(),
            session_id,
            node_id: "node2".to_string(),
            replicated_at: Utc::now(),
            status: ReplicationStatus::Synchronized,
            checksum: "checksum_2".to_string(), // Different checksum
        };

        simulator.session_replications.insert(session_id, vec![replication1, replication2]);

        // Validate consistency
        let inconsistencies = simulator.validate_session_consistency().await;

        assert!(!inconsistencies.is_empty());
        let checksum_mismatch = inconsistencies.iter()
            .find(|(_, reason)| reason == "checksum_mismatch");
        assert!(checksum_mismatch.is_some());
    }

    #[tokio::test]
    async fn test_session_scalability() {
        let mut simulator = SessionManagementSimulator::new();

        // Test creating a moderate number of sessions for scalability
        let duration = simulator.test_session_scalability(50).await.unwrap();

        assert!(duration.num_milliseconds() > 0);
        println!("Created 50 sessions in {} ms", duration.num_milliseconds());

        // Verify sessions were created
        assert!(!simulator.sessions.is_empty());

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert!(metrics.total_concurrent_attempts > 0);
        assert!(metrics.successful_concurrent_sessions > 0);
    }

    #[tokio::test]
    async fn test_session_sync_events() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create session with sync event logging
        let (_, _) = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            "device_1".to_string(),
            false
        ).await.unwrap();

        // Verify sync event was logged
        assert!(!simulator.session_sync_events.is_empty());

        let created_event = simulator.session_sync_events.iter()
            .find(|e| matches!(e.event_type, SessionEventType::Created));
        assert!(created_event.is_some());

        let event = created_event.unwrap();
        assert!(event.data.get("device_id").is_some());
        assert!(event.data.get("ip").is_some());
    }

    #[tokio::test]
    async fn test_session_resource_cleanup() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create expired lock
        let expired_lock = SessionLock {
            session_id: Uuid::now_v7(),
            locked_by: "test".to_string(),
            locked_at: Utc::now() - Duration::hours(1),
            expires_at: Utc::now() - Duration::minutes(30),
            reason: "test".to_string(),
        };
        simulator.session_locks.insert(user_id, expired_lock);

        // Add many sync events
        for i in 0..1500 {
            simulator.session_sync_events.push(SessionSyncEvent {
                id: Uuid::now_v7(),
                session_id: Uuid::now_v7(),
                event_type: SessionEventType::Created,
                timestamp: Utc::now() - Duration::hours(i as i64),
                data: serde_json::json!({}),
            });
        }

        // Add many conflicts to queue
        for i in 0..150 {
            simulator.session_conflict_queue.push_back((Uuid::now_v7(), format!("conflict_{}", i)));
        }

        let initial_events = simulator.session_sync_events.len();
        let initial_conflicts = simulator.session_conflict_queue.len();

        // Run cleanup
        let cleaned = simulator.cleanup_session_resources().await;

        assert!(cleaned > 0);

        // Verify cleanup occurred
        assert!(simulator.session_locks.is_empty()); // Expired lock removed
        assert_eq!(simulator.session_sync_events.len(), 1000); // Trimmed to 1000
        assert_eq!(simulator.session_conflict_queue.len(), 100); // Trimmed to 100

        println!("Cleaned {} resources", cleaned);
        println!("Events: {} -> {}", initial_events, simulator.session_sync_events.len());
        println!("Conflicts: {} -> {}", initial_conflicts, simulator.session_conflict_queue.len());
    }

    #[tokio::test]
    async fn test_ip_based_session_conflicts() {
        let mut simulator = SessionManagementSimulator::new();
        simulator.concurrent_session_policy.allow_concurrent_same_ip = false;
        simulator.concurrent_session_policy.session_collision_strategy = SessionCollisionStrategy::RejectNew;

        let user_id = simulator.create_test_user().await;

        // Create first session from IP
        let _result1 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.100".to_string(),
            "Browser_1".to_string(),
            "device_1".to_string(),
            false
        ).await.unwrap();

        // Try to create second session from same IP (should be rejected)
        let result2 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.100".to_string(),
            "Browser_2".to_string(),
            "device_2".to_string(),
            false
        ).await;

        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("ip_conflict"));

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.collision_resolutions, 1);
    }

    #[tokio::test]
    async fn test_concurrent_session_metrics() {
        let mut simulator = SessionManagementSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Create and end some sessions to test metrics
        let (session_token, _) = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "Browser".to_string(),
            "device_1".to_string(),
            false
        ).await.unwrap();

        // Log a conflict event
        simulator.log_session_sync_event(
            Uuid::now_v7(),
            SessionEventType::Conflicted,
            serde_json::json!({"test": true})
        ).await;

        // End the session
        simulator.logout(&session_token).await.unwrap();

        let metrics = simulator.get_concurrent_session_metrics().await;

        assert_eq!(metrics.total_concurrent_attempts, 1);
        assert_eq!(metrics.successful_concurrent_sessions, 1);
        assert_eq!(metrics.session_conflicts_per_hour, 1.0);
        assert!(metrics.average_session_duration_minutes >= 0.0);
    }

    #[tokio::test]
    async fn test_session_collision_allow_override() {
        let mut simulator = SessionManagementSimulator::new();
        simulator.concurrent_session_policy.session_collision_strategy = SessionCollisionStrategy::AllowOverride;
        simulator.concurrent_session_policy.force_single_device = true;

        let user_id = simulator.create_test_user().await;

        // Create first session
        let result1 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.1".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await;

        assert!(result1.is_ok());

        // Create second session on same device (should be allowed with conflict logged)
        let result2 = simulator.create_concurrent_session(
            user_id,
            "192.168.1.2".to_string(),
            "device_1_Browser".to_string(),
            "device_1".to_string(),
            false
        ).await;

        assert!(result2.is_ok());

        // Verify conflict was logged
        let conflict_event = simulator.session_sync_events.iter()
            .find(|e| matches!(e.event_type, SessionEventType::Conflicted));
        assert!(conflict_event.is_some());

        let metrics = simulator.get_concurrent_session_metrics().await;
        assert_eq!(metrics.collision_resolutions, 1);
    }
}