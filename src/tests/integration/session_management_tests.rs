//! Integration tests for session management and logout operations
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
}