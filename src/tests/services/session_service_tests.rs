//! Tests for Session Management Service
//!
//! This module contains comprehensive tests for session management,
//! including session creation, validation, expiration, and cleanup.

#[cfg(test)]
mod session_service_tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use std::collections::HashMap;

    use crate::application::services::SessionService;
    use crate::errors::AppError;

    // Mock session structure for testing
    #[derive(Debug, Clone)]
    struct TestSession {
        id: Uuid,
        user_id: Uuid,
        token: String,
        device_info: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        expires_at: chrono::DateTime<chrono::Utc>,
        last_activity: chrono::DateTime<chrono::Utc>,
        is_active: bool,
    }

    // Mock session service for testing
    struct TestSessionService {
        sessions: HashMap<String, TestSession>,
        default_session_duration: Duration,
        max_sessions_per_user: usize,
    }

    impl TestSessionService {
        fn new() -> Self {
            Self {
                sessions: HashMap::new(),
                default_session_duration: Duration::hours(24),
                max_sessions_per_user: 5,
            }
        }

        fn create_session(
            &mut self,
            user_id: Uuid,
            device_info: Option<String>,
            ip_address: Option<String>,
            user_agent: Option<String>,
        ) -> Result<TestSession, AppError> {
            let now = Utc::now();
            let session_id = Uuid::new_v4();
            let token = self.generate_session_token();

            let session = TestSession {
                id: session_id,
                user_id,
                token: token.clone(),
                device_info,
                ip_address,
                user_agent,
                created_at: now,
                expires_at: now + self.default_session_duration,
                last_activity: now,
                is_active: true,
            };

            // Check session limit per user
            let user_sessions = self.get_user_sessions(user_id);
            if user_sessions.len() >= self.max_sessions_per_user {
                return Err(AppError::Validation {
                    field: "session".to_string(),
                    message: "Maximum sessions per user exceeded".to_string(),
                });
            }

            self.sessions.insert(token.clone(), session.clone());
            Ok(session)
        }

        fn validate_session(&self, token: &str) -> Result<TestSession, AppError> {
            let session = self.sessions.get(token)
                .ok_or_else(|| AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(token.to_string()),
                })?;

            if !session.is_active {
                return Err(AppError::Authentication {
                    message: "Session is inactive".to_string(),
                });
            }

            if self.is_expired(session) {
                return Err(AppError::Authentication {
                    message: "Session has expired".to_string(),
                });
            }

            Ok(session.clone())
        }

        fn update_activity(&mut self, token: &str) -> Result<(), AppError> {
            let session = self.sessions.get_mut(token)
                .ok_or_else(|| AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(token.to_string()),
                })?;

            session.last_activity = Utc::now();
            Ok(())
        }

        fn extend_session(&mut self, token: &str, additional_duration: Duration) -> Result<(), AppError> {
            let session = self.sessions.get_mut(token)
                .ok_or_else(|| AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(token.to_string()),
                })?;

            session.expires_at = session.expires_at + additional_duration;
            Ok(())
        }

        fn revoke_session(&mut self, token: &str) -> Result<(), AppError> {
            let session = self.sessions.get_mut(token)
                .ok_or_else(|| AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(token.to_string()),
                })?;

            session.is_active = false;
            Ok(())
        }

        fn revoke_all_user_sessions(&mut self, user_id: Uuid) -> Result<i32, AppError> {
            let mut revoked_count = 0;

            for session in self.sessions.values_mut() {
                if session.user_id == user_id && session.is_active {
                    session.is_active = false;
                    revoked_count += 1;
                }
            }

            Ok(revoked_count)
        }

        fn get_user_sessions(&self, user_id: Uuid) -> Vec<TestSession> {
            self.sessions.values()
                .filter(|s| s.user_id == user_id && s.is_active)
                .cloned()
                .collect()
        }

        fn cleanup_expired_sessions(&mut self) -> Result<i32, AppError> {
            let expired_tokens: Vec<String> = self.sessions.iter()
                .filter(|(_, session)| self.is_expired(session))
                .map(|(token, _)| token.clone())
                .collect();

            let count = expired_tokens.len() as i32;
            for token in expired_tokens {
                self.sessions.remove(&token);
            }

            Ok(count)
        }

        fn get_session_info(&self, token: &str) -> Result<TestSession, AppError> {
            self.validate_session(token)
        }

        // Helper methods
        fn generate_session_token(&self) -> String {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..64)
                .map(|_| {
                    let idx = rng.gen_range(0..62);
                    match idx {
                        0..=25 => (b'a' + idx) as char,
                        26..=51 => (b'A' + (idx - 26)) as char,
                        _ => (b'0' + (idx - 52)) as char,
                    }
                })
                .collect()
        }

        fn is_expired(&self, session: &TestSession) -> bool {
            Utc::now() > session.expires_at
        }
    }

    #[test]
    fn test_session_creation() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        let session = service.create_session(
            user_id,
            Some("iPhone 14".to_string()),
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        ).unwrap();

        assert_eq!(session.user_id, user_id);
        assert!(session.is_active);
        assert!(!session.token.is_empty());
        assert_eq!(session.device_info, Some("iPhone 14".to_string()));
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(session.user_agent, Some("Mozilla/5.0".to_string()));
        assert!(session.expires_at > session.created_at);
    }

    #[test]
    fn test_session_validation_success() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        let session = service.create_session(user_id, None, None, None).unwrap();
        let validated = service.validate_session(&session.token);

        assert!(validated.is_ok());
        let validated_session = validated.unwrap();
        assert_eq!(validated_session.id, session.id);
        assert_eq!(validated_session.user_id, user_id);
    }

    #[test]
    fn test_session_validation_not_found() {
        let service = TestSessionService::new();
        let invalid_token = "invalid-token";

        let result = service.validate_session(invalid_token);
        assert!(result.is_err());

        if let Err(AppError::NotFound { resource, .. }) = result {
            assert_eq!(resource, "Session");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_session_expiration() {
        let mut service = TestSessionService::new();
        service.default_session_duration = Duration::seconds(1);

        let user_id = Uuid::new_v4();
        let session = service.create_session(user_id, None, None, None).unwrap();

        // Should be valid initially
        assert!(service.validate_session(&session.token).is_ok());

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should now be expired
        let result = service.validate_session(&session.token);
        assert!(result.is_err());

        if let Err(AppError::Authentication { message }) = result {
            assert!(message.contains("expired"));
        } else {
            panic!("Expected Authentication error for expired session");
        }
    }

    #[test]
    fn test_session_activity_update() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        let session = service.create_session(user_id, None, None, None).unwrap();
        let initial_activity = session.last_activity;

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Update activity
        service.update_activity(&session.token).unwrap();

        let updated_session = service.validate_session(&session.token).unwrap();
        assert!(updated_session.last_activity > initial_activity);
    }

    #[test]
    fn test_session_extension() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        let session = service.create_session(user_id, None, None, None).unwrap();
        let original_expiry = session.expires_at;

        // Extend session by 1 hour
        service.extend_session(&session.token, Duration::hours(1)).unwrap();

        let extended_session = service.validate_session(&session.token).unwrap();
        assert!(extended_session.expires_at > original_expiry);
    }

    #[test]
    fn test_session_revocation() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        let session = service.create_session(user_id, None, None, None).unwrap();

        // Session should be valid initially
        assert!(service.validate_session(&session.token).is_ok());

        // Revoke session
        service.revoke_session(&session.token).unwrap();

        // Session should now be invalid
        let result = service.validate_session(&session.token);
        assert!(result.is_err());

        if let Err(AppError::Authentication { message }) = result {
            assert!(message.contains("inactive"));
        } else {
            panic!("Expected Authentication error for inactive session");
        }
    }

    #[test]
    fn test_revoke_all_user_sessions() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        // Create multiple sessions for the user
        let session1 = service.create_session(user_id, Some("Device 1".to_string()), None, None).unwrap();
        let session2 = service.create_session(user_id, Some("Device 2".to_string()), None, None).unwrap();
        let session3 = service.create_session(user_id, Some("Device 3".to_string()), None, None).unwrap();

        // All sessions should be valid
        assert!(service.validate_session(&session1.token).is_ok());
        assert!(service.validate_session(&session2.token).is_ok());
        assert!(service.validate_session(&session3.token).is_ok());

        // Revoke all sessions for the user
        let revoked_count = service.revoke_all_user_sessions(user_id).unwrap();
        assert_eq!(revoked_count, 3);

        // All sessions should now be invalid
        assert!(service.validate_session(&session1.token).is_err());
        assert!(service.validate_session(&session2.token).is_err());
        assert!(service.validate_session(&session3.token).is_err());
    }

    #[test]
    fn test_user_session_limit() {
        let mut service = TestSessionService::new();
        service.max_sessions_per_user = 2;

        let user_id = Uuid::new_v4();

        // Should be able to create up to the limit
        let session1 = service.create_session(user_id, None, None, None);
        let session2 = service.create_session(user_id, None, None, None);
        assert!(session1.is_ok());
        assert!(session2.is_ok());

        // Third session should fail
        let session3 = service.create_session(user_id, None, None, None);
        assert!(session3.is_err());

        if let Err(AppError::Validation { message, .. }) = session3 {
            assert!(message.contains("Maximum sessions"));
        } else {
            panic!("Expected Validation error for session limit");
        }
    }

    #[test]
    fn test_get_user_sessions() {
        let mut service = TestSessionService::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // Create sessions for different users
        service.create_session(user1, Some("User1 Device1".to_string()), None, None).unwrap();
        service.create_session(user1, Some("User1 Device2".to_string()), None, None).unwrap();
        service.create_session(user2, Some("User2 Device1".to_string()), None, None).unwrap();

        let user1_sessions = service.get_user_sessions(user1);
        let user2_sessions = service.get_user_sessions(user2);

        assert_eq!(user1_sessions.len(), 2);
        assert_eq!(user2_sessions.len(), 1);

        // Verify session ownership
        for session in &user1_sessions {
            assert_eq!(session.user_id, user1);
        }
        for session in &user2_sessions {
            assert_eq!(session.user_id, user2);
        }
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let mut service = TestSessionService::new();
        service.default_session_duration = Duration::seconds(1);

        let user_id = Uuid::new_v4();

        // Create sessions that will expire
        service.create_session(user_id, Some("Expiring1".to_string()), None, None).unwrap();
        service.create_session(user_id, Some("Expiring2".to_string()), None, None).unwrap();

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Create a fresh session
        service.default_session_duration = Duration::hours(1);
        service.create_session(user_id, Some("Fresh".to_string()), None, None).unwrap();

        // Cleanup should remove 2 expired sessions
        let cleaned_count = service.cleanup_expired_sessions().unwrap();
        assert_eq!(cleaned_count, 2);

        // Fresh session should still exist
        let remaining_sessions = service.get_user_sessions(user_id);
        assert_eq!(remaining_sessions.len(), 1);
        assert_eq!(remaining_sessions[0].device_info, Some("Fresh".to_string()));
    }

    #[test]
    fn test_session_token_uniqueness() {
        let mut service = TestSessionService::new();
        service.max_sessions_per_user = 15; // Increase limit for this test

        let user_id = Uuid::new_v4();
        let mut tokens = std::collections::HashSet::new();

        // Create multiple sessions and ensure tokens are unique
        for i in 0..10 {
            let session = service.create_session(
                user_id,
                Some(format!("Device {}", i)),
                None,
                None,
            ).unwrap();

            assert!(!tokens.contains(&session.token), "Token should be unique");
            tokens.insert(session.token);
        }

        assert_eq!(tokens.len(), 10);
    }

    #[test]
    fn test_session_info_retrieval() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();
        let device_info = "Test Device";
        let ip_address = "127.0.0.1";
        let user_agent = "Test Agent";

        let session = service.create_session(
            user_id,
            Some(device_info.to_string()),
            Some(ip_address.to_string()),
            Some(user_agent.to_string()),
        ).unwrap();

        let info = service.get_session_info(&session.token).unwrap();

        assert_eq!(info.user_id, user_id);
        assert_eq!(info.device_info, Some(device_info.to_string()));
        assert_eq!(info.ip_address, Some(ip_address.to_string()));
        assert_eq!(info.user_agent, Some(user_agent.to_string()));
        assert!(info.is_active);
    }

    #[test]
    fn test_concurrent_session_operations() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let service = Arc::new(Mutex::new(TestSessionService::new()));
        let user_id = Uuid::new_v4();
        let mut handles = vec![];

        // Create sessions concurrently
        for i in 0..5 {
            let service_clone = service.clone();
            let handle = thread::spawn(move || {
                let mut service = service_clone.lock().unwrap();
                service.create_session(
                    user_id,
                    Some(format!("Concurrent Device {}", i)),
                    None,
                    None,
                )
            });
            handles.push(handle);
        }

        let mut sessions = vec![];
        for handle in handles {
            let session = handle.join().unwrap().unwrap();
            sessions.push(session);
        }

        // All sessions should have unique tokens
        let tokens: std::collections::HashSet<_> = sessions.iter()
            .map(|s| &s.token)
            .collect();
        assert_eq!(tokens.len(), 5);

        // Verify all sessions can be validated
        let service = service.lock().unwrap();
        for session in &sessions {
            assert!(service.validate_session(&session.token).is_ok());
        }
    }

    #[test]
    fn test_session_edge_cases() {
        let mut service = TestSessionService::new();
        let user_id = Uuid::new_v4();

        // Test with empty device info
        let session = service.create_session(user_id, Some("".to_string()), None, None);
        assert!(session.is_ok());

        // Test with very long device info
        let long_device = "a".repeat(1000);
        let session = service.create_session(user_id, Some(long_device.clone()), None, None);
        assert!(session.is_ok());

        let session = session.unwrap();
        assert_eq!(session.device_info, Some(long_device));

        // Test with special characters in device info
        let special_device = "Device with 特殊字符 and émojis 🚀";
        let session = service.create_session(user_id, Some(special_device.to_string()), None, None);
        assert!(session.is_ok());

        let session = session.unwrap();
        assert_eq!(session.device_info, Some(special_device.to_string()));
    }
}