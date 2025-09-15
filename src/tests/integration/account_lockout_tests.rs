//! Integration tests for account lockout and unlock mechanisms

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
    Locked,
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
    AccountLocked {
        locked_until: DateTime<Utc>,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginAttempt {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub success: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub locked_at: DateTime<Utc>,
    pub locked_until: DateTime<Utc>,
    pub reason: LockoutReason,
    pub locked_by: Option<Uuid>, // Admin who locked the account
    pub unlock_token: Option<String>,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub unlocked_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockoutReason {
    ExcessiveFailedAttempts,
    SuspiciousActivity,
    AdminAction,
    SecurityBreach,
    TermsViolation,
    UserRequest,
}

#[derive(Debug, Clone)]
pub struct UnlockRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub reason: String,
    pub verification_token: String,
    pub verified: bool,
    pub processed_at: Option<DateTime<Utc>>,
    pub approved: Option<bool>,
}

pub struct AccountLockoutSimulator {
    users: Vec<User>,
    login_attempts: Vec<LoginAttempt>,
    lockout_records: HashMap<Uuid, Vec<LockoutRecord>>,
    unlock_requests: HashMap<Uuid, Vec<UnlockRequest>>,
    max_failed_attempts: u32,
    lockout_duration: Duration,
    progressive_lockout: bool,
    ip_based_lockout: HashMap<String, Vec<DateTime<Utc>>>,
    permanent_lockout_threshold: u32,
    unlock_tokens: HashMap<String, Uuid>,
}

impl AccountLockoutSimulator {
    pub fn new() -> Self {
        Self {
            users: vec![],
            login_attempts: vec![],
            lockout_records: HashMap::new(),
            unlock_requests: HashMap::new(),
            max_failed_attempts: 5,
            lockout_duration: Duration::minutes(30),
            progressive_lockout: true,
            ip_based_lockout: HashMap::new(),
            permanent_lockout_threshold: 10,
            unlock_tokens: HashMap::new(),
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

    pub async fn attempt_login(
        &mut self,
        user_id: Uuid,
        password: &str,
        ip_address: String,
        user_agent: String,
    ) -> Result<(), AppError> {
        // Check if user exists and get lock status
        let (is_locked, locked_until, should_unlock, is_suspended) = {
            let user = self.users.iter()
                .find(|u| u.id == user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Check if account is suspended (permanent)
            if user.status == UserStatus::Suspended {
                (true, Some(Utc::now() + Duration::days(36500)), false, true)
            }
            // Check if account is locked
            else if let Some(locked_until) = user.account_locked_until {
                if Utc::now() < locked_until {
                    (true, Some(locked_until), false, false)
                } else {
                    // Lock period has expired
                    (false, None, true, false)
                }
            } else {
                (false, None, false, false)
            }
        };

        if is_locked {
            let reason = if is_suspended {
                "Account has been permanently suspended".to_string()
            } else {
                "Account is temporarily locked due to security reasons".to_string()
            };
            return Err(AppError::AccountLocked {
                locked_until: locked_until.unwrap(),
                reason,
            });
        }

        if should_unlock {
            // Unlock if lock period has expired
            if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
                user.account_locked_until = None;
                user.failed_login_attempts = 0;
                user.status = UserStatus::Active;
            }
        }

        // Check IP-based rate limiting
        self.check_ip_rate_limit(&ip_address)?;

        // Verify password (simplified)
        let success = password == "correct_password";

        // Record login attempt
        let attempt = LoginAttempt {
            id: Uuid::now_v7(),
            user_id,
            timestamp: Utc::now(),
            ip_address: ip_address.clone(),
            user_agent,
            success,
            failure_reason: if !success {
                Some("Invalid password".to_string())
            } else {
                None
            },
        };
        self.login_attempts.push(attempt);

        if success {
            // Reset failed attempts on successful login
            if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
                user.failed_login_attempts = 0;
                user.last_login = Some(Utc::now());
            }
            Ok(())
        } else {
            // Increment failed attempts and get current count
            let failed_attempts = {
                let user = self.users.iter_mut()
                    .find(|u| u.id == user_id)
                    .unwrap();
                user.failed_login_attempts += 1;
                user.failed_login_attempts
            };

            // Track IP-based attempts
            self.ip_based_lockout.entry(ip_address.clone())
                .or_insert_with(Vec::new)
                .push(Utc::now());

            // Check if lockout threshold is reached
            if failed_attempts >= self.max_failed_attempts {
                self.lock_account(user_id, LockoutReason::ExcessiveFailedAttempts, None).await?;
            }

            Err(AppError::Unauthorized {
                message: format!(
                    "Invalid credentials. {} attempts remaining",
                    self.max_failed_attempts.saturating_sub(failed_attempts)
                ),
            })
        }
    }

    pub async fn lock_account(
        &mut self,
        user_id: Uuid,
        reason: LockoutReason,
        admin_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        // Verify user exists
        if !self.users.iter().any(|u| u.id == user_id) {
            return Err(AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            });
        }

        // Calculate lockout duration and check for permanent lockout
        let lockout_duration = if self.progressive_lockout {
            self.calculate_progressive_lockout(user_id)
        } else {
            self.lockout_duration
        };

        let locked_until = Utc::now() + lockout_duration;

        // Check for permanent lockout
        let total_lockouts = self.lockout_records.get(&user_id)
            .map(|records| records.len() as u32)
            .unwrap_or(0);

        // Update user status
        let user = self.users.iter_mut()
            .find(|u| u.id == user_id)
            .unwrap();

        if total_lockouts >= self.permanent_lockout_threshold {
            user.status = UserStatus::Suspended;
            user.account_locked_until = None; // Permanent lock
        } else {
            user.status = UserStatus::Locked;
            user.account_locked_until = Some(locked_until);
        }

        // Create lockout record
        let unlock_token = format!("unlock_{}", Uuid::now_v7());
        let record = LockoutRecord {
            id: Uuid::now_v7(),
            user_id,
            locked_at: Utc::now(),
            locked_until,
            reason,
            locked_by: admin_id,
            unlock_token: Some(unlock_token.clone()),
            unlocked_at: None,
            unlocked_by: None,
        };

        self.lockout_records.entry(user_id)
            .or_insert_with(Vec::new)
            .push(record);

        self.unlock_tokens.insert(unlock_token, user_id);

        Ok(())
    }

    pub async fn unlock_account(
        &mut self,
        user_id: Uuid,
        admin_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let user = self.users.iter_mut()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if user.status != UserStatus::Locked && user.status != UserStatus::Suspended {
            return Err(AppError::BadRequest {
                message: "Account is not locked".to_string(),
            });
        }

        // Unlock the account
        user.status = UserStatus::Active;
        user.account_locked_until = None;
        user.failed_login_attempts = 0;

        // Update lockout record
        if let Some(records) = self.lockout_records.get_mut(&user_id) {
            if let Some(last_record) = records.last_mut() {
                if last_record.unlocked_at.is_none() {
                    last_record.unlocked_at = Some(Utc::now());
                    last_record.unlocked_by = admin_id;
                }
            }
        }

        Ok(())
    }

    pub async fn unlock_with_token(&mut self, token: &str) -> Result<(), AppError> {
        let user_id = self.unlock_tokens.get(token)
            .copied()
            .ok_or_else(|| AppError::Unauthorized {
                message: "Invalid unlock token".to_string(),
            })?;

        self.unlock_account(user_id, None).await?;

        // Remove used token
        self.unlock_tokens.remove(token);

        Ok(())
    }

    pub async fn request_unlock(
        &mut self,
        user_id: Uuid,
        reason: String,
    ) -> Result<String, AppError> {
        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if user.status != UserStatus::Locked && user.status != UserStatus::Suspended {
            return Err(AppError::BadRequest {
                message: "Account is not locked".to_string(),
            });
        }

        let verification_token = format!("verify_{}", Uuid::now_v7());

        let request = UnlockRequest {
            id: Uuid::now_v7(),
            user_id,
            requested_at: Utc::now(),
            reason,
            verification_token: verification_token.clone(),
            verified: false,
            processed_at: None,
            approved: None,
        };

        self.unlock_requests.entry(user_id)
            .or_insert_with(Vec::new)
            .push(request);

        Ok(verification_token)
    }

    pub async fn verify_unlock_request(&mut self, token: &str) -> Result<(), AppError> {
        // Find the request with this token
        for requests in self.unlock_requests.values_mut() {
            if let Some(request) = requests.iter_mut()
                .find(|r| r.verification_token == token && !r.verified)
            {
                request.verified = true;
                return Ok(());
            }
        }

        Err(AppError::NotFound {
            resource: "unlock_request".to_string(),
            id: Some(token.to_string()),
        })
    }

    pub async fn process_unlock_request(
        &mut self,
        request_id: Uuid,
        approve: bool,
        admin_id: Uuid,
    ) -> Result<(), AppError> {
        // Find and update the request
        let mut found = false;
        let mut user_id_to_unlock = None;

        for (user_id, requests) in self.unlock_requests.iter_mut() {
            if let Some(request) = requests.iter_mut().find(|r| r.id == request_id) {
                if !request.verified {
                    return Err(AppError::BadRequest {
                        message: "Request must be verified first".to_string(),
                    });
                }

                if request.processed_at.is_some() {
                    return Err(AppError::BadRequest {
                        message: "Request has already been processed".to_string(),
                    });
                }

                request.processed_at = Some(Utc::now());
                request.approved = Some(approve);
                found = true;

                if approve {
                    user_id_to_unlock = Some(*user_id);
                }
                break;
            }
        }

        if !found {
            return Err(AppError::NotFound {
                resource: "unlock_request".to_string(),
                id: Some(request_id.to_string()),
            });
        }

        // Unlock the account if approved
        if let Some(user_id) = user_id_to_unlock {
            self.unlock_account(user_id, Some(admin_id)).await?;
        }

        Ok(())
    }

    pub async fn get_lockout_status(&self, user_id: Uuid) -> Result<LockoutStatus, AppError> {
        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        let status = LockoutStatus {
            is_locked: user.status == UserStatus::Locked || user.status == UserStatus::Suspended,
            locked_until: user.account_locked_until,
            failed_attempts: user.failed_login_attempts,
            max_attempts: self.max_failed_attempts,
            lockout_history: self.get_lockout_history(user_id),
        };

        Ok(status)
    }

    pub fn get_lockout_history(&self, user_id: Uuid) -> Vec<LockoutRecord> {
        self.lockout_records.get(&user_id)
            .map(|records| records.clone())
            .unwrap_or_default()
    }

    pub async fn clear_failed_attempts(&mut self, user_id: Uuid) -> Result<(), AppError> {
        let user = self.users.iter_mut()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        user.failed_login_attempts = 0;
        Ok(())
    }

    fn calculate_progressive_lockout(&self, user_id: Uuid) -> Duration {
        let lockout_count = self.lockout_records.get(&user_id)
            .map(|records| records.len() as u32)
            .unwrap_or(0);

        // Progressive lockout: 30 min, 1 hour, 2 hours, 4 hours, etc.
        let multiplier = 2_u32.pow(lockout_count.min(5));
        Duration::minutes(30 * multiplier as i64)
    }

    fn check_ip_rate_limit(&mut self, ip_address: &str) -> Result<(), AppError> {
        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);

        // Clean old attempts
        if let Some(attempts) = self.ip_based_lockout.get_mut(ip_address) {
            attempts.retain(|&t| t > hour_ago);

            // Check if IP has too many failed attempts
            if attempts.len() >= 20 {
                return Err(AppError::AccountLocked {
                    locked_until: now + Duration::hours(1),
                    reason: "Too many failed attempts from this IP address".to_string(),
                });
            }
        }

        Ok(())
    }

    pub async fn admin_lock_account(
        &mut self,
        user_id: Uuid,
        admin_id: Uuid,
        reason: LockoutReason,
        duration: Option<Duration>,
    ) -> Result<(), AppError> {
        // Admin can override normal lockout duration
        if let Some(custom_duration) = duration {
            // First verify user exists
            if !self.users.iter().any(|u| u.id == user_id) {
                return Err(AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                });
            }

            let locked_until = Utc::now() + custom_duration;

            // Update user status with custom duration
            let user = self.users.iter_mut()
                .find(|u| u.id == user_id)
                .unwrap();

            user.status = UserStatus::Locked;
            user.account_locked_until = Some(locked_until);

            // Create lockout record with custom duration
            let unlock_token = format!("unlock_{}", Uuid::now_v7());
            let record = LockoutRecord {
                id: Uuid::now_v7(),
                user_id,
                locked_at: Utc::now(),
                locked_until,
                reason,
                locked_by: Some(admin_id),
                unlock_token: Some(unlock_token.clone()),
                unlocked_at: None,
                unlocked_by: None,
            };

            self.lockout_records.entry(user_id)
                .or_insert_with(Vec::new)
                .push(record);

            self.unlock_tokens.insert(unlock_token, user_id);

            Ok(())
        } else {
            self.lock_account(user_id, reason, Some(admin_id)).await
        }
    }

    pub async fn set_permanent_suspension(
        &mut self,
        user_id: Uuid,
        admin_id: Uuid,
        _reason: String,
    ) -> Result<(), AppError> {
        let user = self.users.iter_mut()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        user.status = UserStatus::Suspended;
        user.account_locked_until = None; // Permanent

        // Create lockout record
        let record = LockoutRecord {
            id: Uuid::now_v7(),
            user_id,
            locked_at: Utc::now(),
            locked_until: Utc::now() + Duration::days(36500), // ~100 years
            reason: LockoutReason::AdminAction,
            locked_by: Some(admin_id),
            unlock_token: None,
            unlocked_at: None,
            unlocked_by: None,
        };

        self.lockout_records.entry(user_id)
            .or_insert_with(Vec::new)
            .push(record);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutStatus {
    pub is_locked: bool,
    pub locked_until: Option<DateTime<Utc>>,
    pub failed_attempts: u32,
    pub max_attempts: u32,
    pub lockout_history: Vec<LockoutRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_failed_login_attempts() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;

        // First 4 failed attempts should not lock
        for i in 1..5 {
            let result = simulator.attempt_login(
                user_id,
                "wrong_password",
                format!("192.168.1.{}", i),
                "Mozilla/5.0".to_string()
            ).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), AppError::Unauthorized { .. }));
        }

        // 5th attempt should trigger lockout
        let result = simulator.attempt_login(
            user_id,
            "wrong_password",
            "192.168.1.5".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_err());

        // Next attempt should fail due to lockout
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.6".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::AccountLocked { .. }));
    }

    #[tokio::test]
    async fn test_successful_login_resets_attempts() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Some failed attempts
        for _ in 0..3 {
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                "192.168.1.1".to_string(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        // Successful login
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_ok());

        // Check that failed attempts were reset
        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert_eq!(status.failed_attempts, 0);
    }

    #[tokio::test]
    async fn test_lockout_duration() {
        let mut simulator = AccountLockoutSimulator::new();
        simulator.lockout_duration = Duration::seconds(1); // Short for testing
        let user_id = simulator.create_test_user().await;

        // Trigger lockout
        for _ in 0..5 {
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                "192.168.1.1".to_string(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        // Should be locked
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(matches!(result.unwrap_err(), AppError::AccountLocked { .. }));

        // Manually expire the lockout
        if let Some(user) = simulator.users.iter_mut().find(|u| u.id == user_id) {
            user.account_locked_until = Some(Utc::now() - Duration::hours(1));
        }

        // Should be able to login now
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_admin_unlock() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Lock the account
        simulator.lock_account(user_id, LockoutReason::SuspiciousActivity, Some(admin_id)).await.unwrap();

        // Verify account is locked
        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert!(status.is_locked);

        // Admin unlocks the account
        simulator.unlock_account(user_id, Some(admin_id)).await.unwrap();

        // Verify account is unlocked
        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert!(!status.is_locked);

        // Should be able to login
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unlock_with_token() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Lock the account
        simulator.lock_account(user_id, LockoutReason::ExcessiveFailedAttempts, None).await.unwrap();

        // Get unlock token
        let token = simulator.lockout_records.get(&user_id)
            .and_then(|records| records.last())
            .and_then(|record| record.unlock_token.clone())
            .unwrap();

        // Unlock with token
        simulator.unlock_with_token(&token).await.unwrap();

        // Verify account is unlocked
        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert!(!status.is_locked);
    }

    #[tokio::test]
    async fn test_unlock_request_flow() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Lock the account
        simulator.lock_account(user_id, LockoutReason::ExcessiveFailedAttempts, None).await.unwrap();

        // Request unlock
        let verification_token = simulator.request_unlock(
            user_id,
            "I forgot my password and entered it wrong too many times".to_string()
        ).await.unwrap();

        // Verify the request
        simulator.verify_unlock_request(&verification_token).await.unwrap();

        // Get request ID
        let request_id = simulator.unlock_requests.get(&user_id)
            .and_then(|requests| requests.last())
            .map(|r| r.id)
            .unwrap();

        // Process and approve the request
        simulator.process_unlock_request(request_id, true, admin_id).await.unwrap();

        // Verify account is unlocked
        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert!(!status.is_locked);
    }

    #[tokio::test]
    async fn test_progressive_lockout() {
        let mut simulator = AccountLockoutSimulator::new();
        simulator.progressive_lockout = true;
        simulator.lockout_duration = Duration::minutes(30);
        let user_id = simulator.create_test_user().await;

        // First lockout
        for _ in 0..5 {
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                "192.168.1.1".to_string(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        let history = simulator.get_lockout_history(user_id);
        assert_eq!(history.len(), 1);

        // Unlock and trigger second lockout
        simulator.unlock_account(user_id, None).await.unwrap();

        for _ in 0..5 {
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                "192.168.1.1".to_string(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        let history = simulator.get_lockout_history(user_id);
        assert_eq!(history.len(), 2);

        // Second lockout should have longer duration
        let second_lockout = &history[1];
        let first_lockout = &history[0];
        assert!(second_lockout.locked_until - second_lockout.locked_at >
                first_lockout.locked_until - first_lockout.locked_at);
    }

    #[tokio::test]
    async fn test_ip_based_rate_limiting() {
        let mut simulator = AccountLockoutSimulator::new();
        let ip_address = "192.168.1.100".to_string();

        // Create multiple users
        let mut user_ids = vec![];
        for _ in 0..5 {
            user_ids.push(simulator.create_test_user().await);
        }

        // Try many failed logins from same IP
        for i in 0..20 {
            let user_id = user_ids[i % 5];
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                ip_address.clone(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        // Next attempt from same IP should be blocked
        let result = simulator.attempt_login(
            user_ids[0],
            "correct_password",
            ip_address.clone(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::AccountLocked { .. }));
    }

    #[tokio::test]
    async fn test_permanent_suspension() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Permanently suspend the account
        simulator.set_permanent_suspension(
            user_id,
            admin_id,
            "Terms of service violation".to_string()
        ).await.unwrap();

        // Verify account is suspended
        let user = simulator.users.iter().find(|u| u.id == user_id).unwrap();
        assert_eq!(user.status, UserStatus::Suspended);
        assert!(user.account_locked_until.is_none()); // No expiry

        // Cannot login
        let result = simulator.attempt_login(
            user_id,
            "correct_password",
            "192.168.1.1".to_string(),
            "Mozilla/5.0".to_string()
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_admin_lock_with_custom_duration() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Admin locks with custom 7-day duration
        simulator.admin_lock_account(
            user_id,
            admin_id,
            LockoutReason::TermsViolation,
            Some(Duration::days(7))
        ).await.unwrap();

        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert!(status.is_locked);

        if let Some(locked_until) = status.locked_until {
            let expected = Utc::now() + Duration::days(7);
            assert!((locked_until - expected).num_seconds().abs() < 60);
        }
    }

    #[tokio::test]
    async fn test_lockout_history_tracking() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Multiple lockouts for different reasons
        let reasons = vec![
            LockoutReason::ExcessiveFailedAttempts,
            LockoutReason::SuspiciousActivity,
            LockoutReason::AdminAction,
        ];

        for reason in reasons {
            simulator.lock_account(user_id, reason, Some(admin_id)).await.unwrap();
            simulator.unlock_account(user_id, Some(admin_id)).await.unwrap();
        }

        let history = simulator.get_lockout_history(user_id);
        assert_eq!(history.len(), 3);

        // Verify all lockouts were recorded
        assert!(matches!(history[0].reason, LockoutReason::ExcessiveFailedAttempts));
        assert!(matches!(history[1].reason, LockoutReason::SuspiciousActivity));
        assert!(matches!(history[2].reason, LockoutReason::AdminAction));
    }

    #[tokio::test]
    async fn test_clear_failed_attempts() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;

        // Some failed attempts
        for _ in 0..3 {
            let _ = simulator.attempt_login(
                user_id,
                "wrong_password",
                "192.168.1.1".to_string(),
                "Mozilla/5.0".to_string()
            ).await;
        }

        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert_eq!(status.failed_attempts, 3);

        // Clear failed attempts
        simulator.clear_failed_attempts(user_id).await.unwrap();

        let status = simulator.get_lockout_status(user_id).await.unwrap();
        assert_eq!(status.failed_attempts, 0);
    }

    #[tokio::test]
    async fn test_reject_unverified_unlock_request() {
        let mut simulator = AccountLockoutSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Lock the account
        simulator.lock_account(user_id, LockoutReason::ExcessiveFailedAttempts, None).await.unwrap();

        // Request unlock but don't verify
        let _ = simulator.request_unlock(
            user_id,
            "Please unlock my account".to_string()
        ).await.unwrap();

        // Get request ID
        let request_id = simulator.unlock_requests.get(&user_id)
            .and_then(|requests| requests.last())
            .map(|r| r.id)
            .unwrap();

        // Try to process without verification
        let result = simulator.process_unlock_request(request_id, true, admin_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest { .. }));
    }
}