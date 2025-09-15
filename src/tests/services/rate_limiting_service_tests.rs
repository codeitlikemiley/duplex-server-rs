//! Tests for Rate Limiting Service
//!
//! This module contains comprehensive tests for rate limiting logic,
//! including request tracking, threshold enforcement, and window management.

#[cfg(test)]
mod rate_limiting_service_tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use std::collections::HashMap;
    use std::net::IpAddr;

    use crate::application::services::RateLimitingService;
    use crate::errors::AppError;

    // Rate limit types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum RateLimitType {
        Login,
        Registration,
        PasswordReset,
        EmailVerification,
        ApiRequest,
        FileUpload,
    }

    // Rate limit configuration
    #[derive(Debug, Clone)]
    struct RateLimitConfig {
        max_attempts: u32,
        window_duration: Duration,
        block_duration: Option<Duration>,
    }

    // Request tracking structure
    #[derive(Debug, Clone)]
    struct RequestTracker {
        count: u32,
        window_start: chrono::DateTime<chrono::Utc>,
        last_request: chrono::DateTime<chrono::Utc>,
        blocked_until: Option<chrono::DateTime<chrono::Utc>>,
    }

    // Mock rate limiting service for testing
    struct TestRateLimitingService {
        configs: HashMap<RateLimitType, RateLimitConfig>,
        ip_trackers: HashMap<(IpAddr, RateLimitType), RequestTracker>,
        user_trackers: HashMap<(Uuid, RateLimitType), RequestTracker>,
        global_trackers: HashMap<RateLimitType, RequestTracker>,
    }

    impl TestRateLimitingService {
        fn new() -> Self {
            let mut configs = HashMap::new();

            // Default configurations
            configs.insert(RateLimitType::Login, RateLimitConfig {
                max_attempts: 5,
                window_duration: Duration::minutes(15),
                block_duration: Some(Duration::hours(1)),
            });

            configs.insert(RateLimitType::Registration, RateLimitConfig {
                max_attempts: 3,
                window_duration: Duration::hours(1),
                block_duration: Some(Duration::hours(24)),
            });

            configs.insert(RateLimitType::PasswordReset, RateLimitConfig {
                max_attempts: 3,
                window_duration: Duration::hours(1),
                block_duration: None,
            });

            configs.insert(RateLimitType::EmailVerification, RateLimitConfig {
                max_attempts: 5,
                window_duration: Duration::minutes(30),
                block_duration: None,
            });

            configs.insert(RateLimitType::ApiRequest, RateLimitConfig {
                max_attempts: 100,
                window_duration: Duration::minutes(1),
                block_duration: None,
            });

            configs.insert(RateLimitType::FileUpload, RateLimitConfig {
                max_attempts: 10,
                window_duration: Duration::minutes(10),
                block_duration: Some(Duration::minutes(30)),
            });

            Self {
                configs,
                ip_trackers: HashMap::new(),
                user_trackers: HashMap::new(),
                global_trackers: HashMap::new(),
            }
        }

        fn check_rate_limit_by_ip(
            &mut self,
            ip: IpAddr,
            limit_type: RateLimitType,
        ) -> Result<(), AppError> {
            let config = self.configs.get(&limit_type)
                .ok_or_else(|| AppError::Internal {
                    message: "Rate limit config not found".to_string(),
                })?;

            let now = Utc::now();
            let key = (ip, limit_type);

            let tracker = self.ip_trackers.entry(key).or_insert_with(|| {
                RequestTracker {
                    count: 0,
                    window_start: now,
                    last_request: now,
                    blocked_until: None,
                }
            });

            Self::check_and_update_tracker(tracker, config, now)
        }

        fn check_rate_limit_by_user(
            &mut self,
            user_id: Uuid,
            limit_type: RateLimitType,
        ) -> Result<(), AppError> {
            let config = self.configs.get(&limit_type)
                .ok_or_else(|| AppError::Internal {
                    message: "Rate limit config not found".to_string(),
                })?;

            let now = Utc::now();
            let key = (user_id, limit_type);

            let tracker = self.user_trackers.entry(key).or_insert_with(|| {
                RequestTracker {
                    count: 0,
                    window_start: now,
                    last_request: now,
                    blocked_until: None,
                }
            });

            Self::check_and_update_tracker(tracker, config, now)
        }

        fn check_global_rate_limit(
            &mut self,
            limit_type: RateLimitType,
        ) -> Result<(), AppError> {
            let config = self.configs.get(&limit_type)
                .ok_or_else(|| AppError::Internal {
                    message: "Rate limit config not found".to_string(),
                })?;

            let now = Utc::now();

            let tracker = self.global_trackers.entry(limit_type).or_insert_with(|| {
                RequestTracker {
                    count: 0,
                    window_start: now,
                    last_request: now,
                    blocked_until: None,
                }
            });

            Self::check_and_update_tracker(tracker, config, now)
        }

        fn check_and_update_tracker(
            tracker: &mut RequestTracker,
            config: &RateLimitConfig,
            now: chrono::DateTime<chrono::Utc>,
        ) -> Result<(), AppError> {
            // Check if currently blocked
            if let Some(blocked_until) = tracker.blocked_until {
                if now < blocked_until {
                    let remaining = (blocked_until - now).num_seconds();
                    return Err(AppError::Validation {
                        field: "rate_limit".to_string(),
                        message: format!("Rate limited. Try again in {} seconds", remaining),
                    });
                }
                // Block expired, reset tracker
                tracker.blocked_until = None;
                tracker.count = 0;
                tracker.window_start = now;
            }

            // Check if window has expired
            if now > tracker.window_start + config.window_duration {
                // Reset window
                tracker.count = 0;
                tracker.window_start = now;
            }

            // Increment count
            tracker.count += 1;
            tracker.last_request = now;

            // Check if limit exceeded
            if tracker.count > config.max_attempts {
                // Apply block if configured
                if let Some(block_duration) = config.block_duration {
                    tracker.blocked_until = Some(now + block_duration);
                }

                return Err(AppError::Validation {
                    field: "rate_limit".to_string(),
                    message: format!("Rate limit exceeded. Maximum {} attempts allowed", config.max_attempts),
                });
            }

            Ok(())
        }

        fn reset_ip_limit(&mut self, ip: IpAddr, limit_type: RateLimitType) {
            self.ip_trackers.remove(&(ip, limit_type));
        }

        fn reset_user_limit(&mut self, user_id: Uuid, limit_type: RateLimitType) {
            self.user_trackers.remove(&(user_id, limit_type));
        }

        fn get_remaining_attempts_for_ip(&self, ip: IpAddr, limit_type: RateLimitType) -> Option<u32> {
            let config = self.configs.get(&limit_type)?;
            let tracker = self.ip_trackers.get(&(ip, limit_type))?;

            let now = Utc::now();
            if now > tracker.window_start + config.window_duration {
                // Window expired, would reset on next request
                return Some(config.max_attempts);
            }

            Some(config.max_attempts.saturating_sub(tracker.count))
        }

        fn get_remaining_attempts_for_user(&self, user_id: Uuid, limit_type: RateLimitType) -> Option<u32> {
            let config = self.configs.get(&limit_type)?;
            let tracker = self.user_trackers.get(&(user_id, limit_type))?;

            let now = Utc::now();
            if now > tracker.window_start + config.window_duration {
                // Window expired, would reset on next request
                return Some(config.max_attempts);
            }

            Some(config.max_attempts.saturating_sub(tracker.count))
        }

        fn is_blocked_ip(&self, ip: IpAddr, limit_type: RateLimitType) -> bool {
            if let Some(tracker) = self.ip_trackers.get(&(ip, limit_type)) {
                if let Some(blocked_until) = tracker.blocked_until {
                    return Utc::now() < blocked_until;
                }
            }
            false
        }

        fn cleanup_expired_trackers(&mut self) -> u32 {
            let now = Utc::now();
            let mut removed = 0;

            // Clean IP trackers
            self.ip_trackers.retain(|_, tracker| {
                let should_keep = if let Some(blocked_until) = tracker.blocked_until {
                    now < blocked_until + Duration::hours(1) // Keep for 1 hour after block expires
                } else {
                    now < tracker.last_request + Duration::hours(24) // Keep for 24 hours after last request
                };
                if !should_keep {
                    removed += 1;
                }
                should_keep
            });

            // Clean user trackers
            self.user_trackers.retain(|_, tracker| {
                let should_keep = now < tracker.last_request + Duration::hours(24);
                if !should_keep {
                    removed += 1;
                }
                should_keep
            });

            removed
        }

        fn update_config(&mut self, limit_type: RateLimitType, config: RateLimitConfig) {
            self.configs.insert(limit_type, config);
        }
    }

    #[test]
    fn test_basic_rate_limiting() {
        let mut service = TestRateLimitingService::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // First 5 login attempts should succeed
        for i in 1..=5 {
            let result = service.check_rate_limit_by_ip(ip, RateLimitType::Login);
            assert!(result.is_ok(), "Attempt {} should succeed", i);
        }

        // 6th attempt should fail
        let result = service.check_rate_limit_by_ip(ip, RateLimitType::Login);
        assert!(result.is_err());

        if let Err(AppError::Validation { message, .. }) = result {
            assert!(message.contains("Rate limit exceeded"));
        } else {
            panic!("Expected rate limit error");
        }
    }

    #[test]
    fn test_window_reset() {
        let mut service = TestRateLimitingService::new();
        service.update_config(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::seconds(1),
            block_duration: None,
        });

        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Use up the limit
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_err());

        // Wait for window to reset
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should be able to make requests again
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
    }

    #[test]
    fn test_blocking_duration() {
        let mut service = TestRateLimitingService::new();
        service.update_config(RateLimitType::Login, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(15),
            block_duration: Some(Duration::seconds(2)),
        });

        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Exceed the limit
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_err());

        // Should still be blocked
        assert!(service.is_blocked_ip(ip, RateLimitType::Login));
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_err());

        // Wait for block to expire
        std::thread::sleep(std::time::Duration::from_secs(3));

        // Should no longer be blocked
        assert!(!service.is_blocked_ip(ip, RateLimitType::Login));
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_ok());
    }

    #[test]
    fn test_user_based_rate_limiting() {
        let mut service = TestRateLimitingService::new();
        let user_id = Uuid::new_v4();

        // Configure password reset limits
        service.update_config(RateLimitType::PasswordReset, RateLimitConfig {
            max_attempts: 3,
            window_duration: Duration::hours(1),
            block_duration: None,
        });

        // First 3 attempts should succeed
        for _ in 0..3 {
            assert!(service.check_rate_limit_by_user(user_id, RateLimitType::PasswordReset).is_ok());
        }

        // 4th attempt should fail
        assert!(service.check_rate_limit_by_user(user_id, RateLimitType::PasswordReset).is_err());
    }

    #[test]
    fn test_different_limit_types_independent() {
        let mut service = TestRateLimitingService::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Max out login attempts
        for _ in 0..5 {
            service.check_rate_limit_by_ip(ip, RateLimitType::Login).ok();
        }
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_err());

        // Registration should still work (different limit type)
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Registration).is_ok());

        // API requests should also work
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
    }

    #[test]
    fn test_different_ips_independent() {
        let mut service = TestRateLimitingService::new();
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        service.update_config(RateLimitType::Login, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(15),
            block_duration: None,
        });

        // Max out IP1
        assert!(service.check_rate_limit_by_ip(ip1, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ip1, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ip1, RateLimitType::Login).is_err());

        // IP2 should still work
        assert!(service.check_rate_limit_by_ip(ip2, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ip2, RateLimitType::Login).is_ok());
    }

    #[test]
    fn test_remaining_attempts_calculation() {
        let mut service = TestRateLimitingService::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        service.update_config(RateLimitType::EmailVerification, RateLimitConfig {
            max_attempts: 5,
            window_duration: Duration::minutes(30),
            block_duration: None,
        });

        // Initially should have all attempts
        assert_eq!(service.get_remaining_attempts_for_ip(ip, RateLimitType::EmailVerification), None);

        // Use one attempt
        service.check_rate_limit_by_ip(ip, RateLimitType::EmailVerification).unwrap();
        assert_eq!(service.get_remaining_attempts_for_ip(ip, RateLimitType::EmailVerification), Some(4));

        // Use two more
        service.check_rate_limit_by_ip(ip, RateLimitType::EmailVerification).unwrap();
        service.check_rate_limit_by_ip(ip, RateLimitType::EmailVerification).unwrap();
        assert_eq!(service.get_remaining_attempts_for_ip(ip, RateLimitType::EmailVerification), Some(2));
    }

    #[test]
    fn test_reset_limits() {
        let mut service = TestRateLimitingService::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let user_id = Uuid::new_v4();

        // Use up some attempts
        for _ in 0..3 {
            service.check_rate_limit_by_ip(ip, RateLimitType::Login).unwrap();
            service.check_rate_limit_by_user(user_id, RateLimitType::PasswordReset).unwrap();
        }

        // Reset IP limit
        service.reset_ip_limit(ip, RateLimitType::Login);
        assert_eq!(service.get_remaining_attempts_for_ip(ip, RateLimitType::Login), None);

        // Reset user limit
        service.reset_user_limit(user_id, RateLimitType::PasswordReset);
        assert_eq!(service.get_remaining_attempts_for_user(user_id, RateLimitType::PasswordReset), None);

        // Should be able to use again
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_user(user_id, RateLimitType::PasswordReset).is_ok());
    }

    #[test]
    fn test_global_rate_limiting() {
        let mut service = TestRateLimitingService::new();

        service.update_config(RateLimitType::Registration, RateLimitConfig {
            max_attempts: 3,
            window_duration: Duration::minutes(1),
            block_duration: None,
        });

        // Global limit should apply regardless of IP
        for _ in 0..3 {
            assert!(service.check_global_rate_limit(RateLimitType::Registration).is_ok());
        }

        // Should be exceeded
        assert!(service.check_global_rate_limit(RateLimitType::Registration).is_err());
    }

    #[test]
    fn test_cleanup_expired_trackers() {
        let mut service = TestRateLimitingService::new();
        let old_ip: IpAddr = "10.0.0.1".parse().unwrap();
        let recent_ip: IpAddr = "10.0.0.2".parse().unwrap();

        // Create an old tracker
        service.check_rate_limit_by_ip(old_ip, RateLimitType::Login).unwrap();

        // Manually age the tracker
        if let Some(tracker) = service.ip_trackers.get_mut(&(old_ip, RateLimitType::Login)) {
            tracker.last_request = Utc::now() - Duration::hours(25);
        }

        // Create a recent tracker
        service.check_rate_limit_by_ip(recent_ip, RateLimitType::Login).unwrap();

        // Run cleanup
        let removed = service.cleanup_expired_trackers();
        assert_eq!(removed, 1);

        // Old tracker should be gone
        assert!(!service.ip_trackers.contains_key(&(old_ip, RateLimitType::Login)));

        // Recent tracker should remain
        assert!(service.ip_trackers.contains_key(&(recent_ip, RateLimitType::Login)));
    }

    #[test]
    fn test_concurrent_rate_limiting() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let service = Arc::new(Mutex::new(TestRateLimitingService::new()));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Update config for testing
        {
            let mut service = service.lock().unwrap();
            service.update_config(RateLimitType::ApiRequest, RateLimitConfig {
                max_attempts: 10,
                window_duration: Duration::seconds(10),
                block_duration: None,
            });
        }

        let mut handles = vec![];

        // Spawn threads to make concurrent requests
        for _ in 0..15 {
            let service_clone = service.clone();
            let handle = thread::spawn(move || {
                let mut service = service_clone.lock().unwrap();
                service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest)
            });
            handles.push(handle);
        }

        let mut successes = 0;
        let mut failures = 0;

        for handle in handles {
            let result = handle.join().unwrap();
            if result.is_ok() {
                successes += 1;
            } else {
                failures += 1;
            }
        }

        // Should have exactly 10 successes (the limit)
        assert_eq!(successes, 10);
        assert_eq!(failures, 5);
    }

    #[test]
    fn test_ipv6_rate_limiting() {
        let mut service = TestRateLimitingService::new();
        let ipv6: IpAddr = "2001:db8::1".parse().unwrap();

        service.update_config(RateLimitType::Login, RateLimitConfig {
            max_attempts: 2,
            window_duration: Duration::minutes(15),
            block_duration: None,
        });

        // IPv6 should work the same as IPv4
        assert!(service.check_rate_limit_by_ip(ipv6, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ipv6, RateLimitType::Login).is_ok());
        assert!(service.check_rate_limit_by_ip(ipv6, RateLimitType::Login).is_err());
    }

    #[test]
    fn test_edge_cases() {
        let mut service = TestRateLimitingService::new();

        // Test with zero max attempts
        service.update_config(RateLimitType::FileUpload, RateLimitConfig {
            max_attempts: 0,
            window_duration: Duration::minutes(1),
            block_duration: None,
        });

        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::FileUpload).is_err());

        // Test with very short window
        service.update_config(RateLimitType::ApiRequest, RateLimitConfig {
            max_attempts: 1,
            window_duration: Duration::milliseconds(100),
            block_duration: None,
        });

        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_err());

        // Wait for window to expire
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(service.check_rate_limit_by_ip(ip, RateLimitType::ApiRequest).is_ok());
    }
}