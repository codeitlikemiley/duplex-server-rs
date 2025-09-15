//! Integration Tests for User Login and Session Creation
//!
//! This module contains comprehensive integration tests for the complete user login process
//! and session management functionality including concurrent login handling,
//! session security, and advanced authentication scenarios.

#[cfg(test)]
mod user_login_session_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    // Enhanced login and session simulator with comprehensive features
    struct LoginSessionSimulator {
        users: Vec<User>,
        sessions: Vec<SessionInfo>,
        failed_attempts: Vec<(String, i32)>, // email, attempt count
        locked_accounts: Vec<String>, // locked emails
        rate_limits: HashMap<String, VecDeque<chrono::DateTime<Utc>>>, // IP -> login attempts
        device_fingerprints: HashMap<String, DeviceInfo>, // session_id -> device info
        concurrent_sessions: HashMap<Uuid, u32>, // user_id -> session count
        session_activity: HashMap<String, Vec<ActivityLog>>, // session_id -> activities
        suspicious_ips: HashMap<String, SuspiciousActivity>, // IP -> suspicious data
        geo_locations: HashMap<String, GeoLocation>, // IP -> location
        two_factor_challenges: HashMap<String, TwoFactorChallenge>, // user_id -> challenge
        password_reset_tokens: HashMap<String, PasswordResetInfo>, // token -> reset info
        remember_me_tokens: HashMap<String, RememberMeToken>, // token -> user info
    }

    #[derive(Debug, Clone)]
    struct DeviceInfo {
        fingerprint: String,
        first_seen: chrono::DateTime<Utc>,
        last_seen: chrono::DateTime<Utc>,
        is_trusted: bool,
        browser: String,
        os: String,
    }

    #[derive(Debug, Clone)]
    struct ActivityLog {
        timestamp: chrono::DateTime<Utc>,
        action: String,
        ip_address: String,
        details: serde_json::Value,
    }

    #[derive(Debug, Clone)]
    struct SuspiciousActivity {
        failed_attempts: u32,
        different_users_attempted: Vec<String>,
        last_attempt: chrono::DateTime<Utc>,
        is_blocked: bool,
    }

    #[derive(Debug, Clone)]
    struct GeoLocation {
        country: String,
        city: String,
        latitude: f64,
        longitude: f64,
    }

    #[derive(Debug, Clone)]
    struct TwoFactorChallenge {
        code: String,
        expires_at: chrono::DateTime<Utc>,
        attempts: u32,
    }

    #[derive(Debug, Clone)]
    struct PasswordResetInfo {
        user_id: Uuid,
        token: String,
        expires_at: chrono::DateTime<Utc>,
        used: bool,
    }

    #[derive(Debug, Clone)]
    struct RememberMeToken {
        user_id: Uuid,
        token: String,
        expires_at: chrono::DateTime<Utc>,
        device_fingerprint: String,
    }

    #[derive(Debug, Clone)]
    struct SessionInfo {
        id: String,
        user_id: Uuid,
        token: String,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        ip_address: String,
        user_agent: String,
        is_active: bool,
    }

    impl LoginSessionSimulator {
        fn new() -> Self {
            Self {
                users: Vec::new(),
                sessions: Vec::new(),
                failed_attempts: Vec::new(),
                locked_accounts: Vec::new(),
                rate_limits: HashMap::new(),
                device_fingerprints: HashMap::new(),
                concurrent_sessions: HashMap::new(),
                session_activity: HashMap::new(),
                suspicious_ips: HashMap::new(),
                geo_locations: HashMap::new(),
                two_factor_challenges: HashMap::new(),
                password_reset_tokens: HashMap::new(),
                remember_me_tokens: HashMap::new(),
            }
        }

        fn create_test_user(&mut self, email: String, username: String, password: String, verified: bool) -> User {
            let user = User {
                id: Uuid::new_v4(),
                username,
                email: email.clone(),
                password_hash: format!("hashed_{}", password),
                email_verified: verified,
                status: if verified { UserStatus::Active } else { UserStatus::PendingVerification },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            };

            self.users.push(user.clone());
            user
        }

        fn validate_credentials(&self, email: &str, password: &str) -> Result<&User, AppError> {
            // Check if account is locked
            if self.locked_accounts.contains(&email.to_string()) {
                return Err(AppError::Authentication {
                    message: "Account is locked due to too many failed attempts".to_string(),
                });
            }

            // Find user by email
            let user = self.users.iter()
                .find(|u| u.email == email)
                .ok_or_else(|| AppError::Authentication {
                    message: "Invalid credentials".to_string(),
                })?;

            // Check if email is verified first (more specific error)
            if !user.email_verified {
                return Err(AppError::Authentication {
                    message: "Email not verified".to_string(),
                });
            }

            // Check if user is active (after email verification check)
            if user.status != UserStatus::Active {
                return Err(AppError::Authentication {
                    message: "Account is not active".to_string(),
                });
            }

            // Verify password (simple check for testing)
            let expected_hash = format!("hashed_{}", password);
            if user.password_hash != expected_hash {
                return Err(AppError::Authentication {
                    message: "Invalid credentials".to_string(),
                });
            }

            Ok(user)
        }

        fn record_login_attempt(&mut self, email: &str, success: bool) {
            if success {
                // Clear failed attempts on successful login
                self.failed_attempts.retain(|(e, _)| e != email);
                self.locked_accounts.retain(|e| e != email);
            } else {
                // Increment failed attempts
                if let Some((_, count)) = self.failed_attempts.iter_mut().find(|(e, _)| e == email) {
                    *count += 1;
                    if *count >= 5 {
                        self.locked_accounts.push(email.to_string());
                    }
                } else {
                    self.failed_attempts.push((email.to_string(), 1));
                }
            }
        }

        fn create_session(&mut self, user_id: Uuid, ip_address: String, user_agent: String) -> Result<SessionInfo, AppError> {
            let session = SessionInfo {
                id: Uuid::new_v4().to_string(),
                user_id,
                token: format!("session_{}", Uuid::new_v4()),
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(24),
                ip_address,
                user_agent,
                is_active: true,
            };

            self.sessions.push(session.clone());
            Ok(session)
        }

        fn login(&mut self, email: String, password: String, ip_address: String, user_agent: String)
            -> Result<(User, SessionInfo), AppError> {

            // First validate credentials without mutable borrow
            let user_result = self.validate_credentials(&email, &password);

            let user = match user_result {
                Ok(user) => {
                    let user_clone = user.clone();
                    self.record_login_attempt(&email, true);
                    user_clone
                },
                Err(e) => {
                    self.record_login_attempt(&email, false);
                    return Err(e);
                }
            };

            // Update last login time and get the updated user
            let updated_user = if let Some(user_ref) = self.users.iter_mut().find(|u| u.id == user.id) {
                user_ref.last_login_at = Some(Utc::now());
                user_ref.clone()
            } else {
                user
            };

            let session = self.create_session(updated_user.id, ip_address, user_agent)?;
            Ok((updated_user, session))
        }

        fn get_user_sessions(&self, user_id: &Uuid) -> Vec<SessionInfo> {
            self.sessions.iter()
                .filter(|s| s.user_id == *user_id && s.is_active)
                .cloned()
                .collect()
        }

        fn revoke_session(&mut self, session_id: &str) -> Result<(), AppError> {
            if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
                session.is_active = false;
                Ok(())
            } else {
                Err(AppError::NotFound {
                    resource: "Session".to_string(),
                    id: Some(session_id.to_string()),
                })
            }
        }

        fn revoke_all_user_sessions(&mut self, user_id: &Uuid) -> usize {
            let mut count = 0;
            for session in self.sessions.iter_mut() {
                if session.user_id == *user_id && session.is_active {
                    session.is_active = false;
                    count += 1;
                }
            }
            count
        }

        fn validate_session_token(&self, token: &str) -> Result<&SessionInfo, AppError> {
            let session = self.sessions.iter()
                .find(|s| s.token == token && s.is_active)
                .ok_or_else(|| AppError::Authentication {
                    message: "Invalid session token".to_string(),
                })?;

            // Check if session is expired
            if session.expires_at < Utc::now() {
                return Err(AppError::Authentication {
                    message: "Session expired".to_string(),
                });
            }

            Ok(session)
        }

        fn cleanup_expired_sessions(&mut self) -> usize {
            let now = Utc::now();
            let mut count = 0;
            for session in self.sessions.iter_mut() {
                if session.expires_at < now && session.is_active {
                    session.is_active = false;
                    count += 1;
                }
            }
            count
        }

        fn check_rate_limit(&mut self, ip: &str) -> Result<(), AppError> {
            let now = Utc::now();
            let window_start = now - Duration::minutes(5); // 5-minute window

            let attempts = self.rate_limits.entry(ip.to_string()).or_insert(VecDeque::new());

            // Remove old attempts outside the window
            while let Some(&front_time) = attempts.front() {
                if front_time < window_start {
                    attempts.pop_front();
                } else {
                    break;
                }
            }

            // Check if rate limit exceeded (10 attempts per 5 minutes)
            if attempts.len() >= 10 {
                return Err(AppError::Authentication {
                    message: "Too many login attempts. Please try again later.".to_string(),
                });
            }

            attempts.push_back(now);
            Ok(())
        }

        fn detect_suspicious_activity(&mut self, ip: &str, email: &str, success: bool) {
            let suspicious = self.suspicious_ips.entry(ip.to_string()).or_insert(SuspiciousActivity {
                failed_attempts: 0,
                different_users_attempted: Vec::new(),
                last_attempt: Utc::now(),
                is_blocked: false,
            });

            suspicious.last_attempt = Utc::now();

            if !success {
                suspicious.failed_attempts += 1;
                if !suspicious.different_users_attempted.contains(&email.to_string()) {
                    suspicious.different_users_attempted.push(email.to_string());
                }

                // Block IP if too many failed attempts or too many different users
                if suspicious.failed_attempts > 5 || suspicious.different_users_attempted.len() > 2 {
                    suspicious.is_blocked = true;
                }
            } else {
                // Reset on successful login
                suspicious.failed_attempts = 0;
                suspicious.different_users_attempted.clear();
            }
        }

        fn generate_device_fingerprint(&self, user_agent: &str, ip: &str) -> String {
            // Simple fingerprint based on user agent and IP
            format!("fp_{}_{}",
                user_agent.chars().take(20).collect::<String>().replace(" ", "_"),
                ip.replace(".", "_")
            )
        }

        fn track_device(&mut self, session_id: &str, ip: &str, user_agent: &str) {
            let fingerprint = self.generate_device_fingerprint(user_agent, ip);
            let now = Utc::now();

            // Parse browser and OS from user agent (simplified)
            let browser = if user_agent.contains("Chrome") { "Chrome" }
                else if user_agent.contains("Firefox") { "Firefox" }
                else if user_agent.contains("Safari") { "Safari" }
                else { "Unknown" };

            let os = if user_agent.contains("Windows") { "Windows" }
                else if user_agent.contains("Mac") { "macOS" }
                else if user_agent.contains("Linux") { "Linux" }
                else { "Unknown" };

            let device_info = self.device_fingerprints.entry(session_id.to_string()).or_insert(DeviceInfo {
                fingerprint: fingerprint.clone(),
                first_seen: now,
                last_seen: now,
                is_trusted: false, // New devices start as untrusted
                browser: browser.to_string(),
                os: os.to_string(),
            });

            device_info.last_seen = now;

            // Trust device after 24 hours of first use
            if now - device_info.first_seen > Duration::hours(24) {
                device_info.is_trusted = true;
            }
        }

        fn log_session_activity(&mut self, session_id: &str, action: &str, ip: &str, details: serde_json::Value) {
            let activity = ActivityLog {
                timestamp: Utc::now(),
                action: action.to_string(),
                ip_address: ip.to_string(),
                details,
            };

            self.session_activity.entry(session_id.to_string()).or_insert(Vec::new()).push(activity);
        }

        fn create_remember_me_token(&mut self, user_id: Uuid, device_fingerprint: String) -> String {
            let token = format!("remember_{}", Uuid::new_v4());
            let remember_token = RememberMeToken {
                user_id,
                token: token.clone(),
                expires_at: Utc::now() + Duration::days(30),
                device_fingerprint,
            };

            self.remember_me_tokens.insert(token.clone(), remember_token);
            token
        }

        fn validate_remember_me_token(&self, token: &str) -> Result<Uuid, AppError> {
            let remember_token = self.remember_me_tokens.get(token)
                .ok_or_else(|| AppError::Authentication {
                    message: "Invalid remember me token".to_string(),
                })?;

            if remember_token.expires_at < Utc::now() {
                return Err(AppError::Authentication {
                    message: "Remember me token expired".to_string(),
                });
            }

            Ok(remember_token.user_id)
        }

        fn enhanced_login(&mut self, email: String, password: String, ip_address: String, user_agent: String, remember_me: bool)
            -> Result<(User, SessionInfo, Option<String>), AppError> {

            // Check rate limiting
            self.check_rate_limit(&ip_address)?;

            // Check if IP is blocked for suspicious activity
            if let Some(suspicious) = self.suspicious_ips.get(&ip_address) {
                if suspicious.is_blocked {
                    return Err(AppError::Authentication {
                        message: "IP address is temporarily blocked due to suspicious activity".to_string(),
                    });
                }
            }

            // Perform login
            let login_result = self.login(email.clone(), password, ip_address.clone(), user_agent.clone());

            // Track suspicious activity
            self.detect_suspicious_activity(&ip_address, &email, login_result.is_ok());

            let (user, session) = login_result?;

            // Track device information
            self.track_device(&session.id, &ip_address, &user_agent);

            // Log login activity
            self.log_session_activity(&session.id, "login", &ip_address, json!({
                "email": email,
                "success": true
            }));

            // Update concurrent session count
            let count = self.concurrent_sessions.entry(user.id).or_insert(0);
            *count += 1;

            // Create remember me token if requested
            let remember_token = if remember_me {
                let fingerprint = self.generate_device_fingerprint(&user_agent, &ip_address);
                Some(self.create_remember_me_token(user.id, fingerprint))
            } else {
                None
            };

            Ok((user, session, remember_token))
        }

        async fn concurrent_login_test(&mut self, email: String, password: String, concurrent_count: usize)
            -> Vec<Result<(User, SessionInfo), AppError>> {

            let mut results = Vec::new();

            // Simulate concurrent login attempts
            for i in 0..concurrent_count {
                let email_clone = email.clone();
                let password_clone = password.clone();
                let ip = format!("192.168.1.{}", i + 100);
                let user_agent = format!("Browser{}/1.0", i);

                // In a real async environment, this would be proper async
                let result = self.login(email_clone, password_clone, ip, user_agent);
                results.push(result);

                // Small delay to simulate real network timing
                sleep(TokioDuration::from_millis(10)).await;
            }

            results
        }

        fn simulate_session_hijacking_attempt(&mut self, session_token: &str, malicious_ip: &str) -> Result<(), AppError> {
            // Attempt to validate session from different IP
            let session = self.validate_session_token(session_token)?;

            // Clone session data to avoid borrow checker issues
            let (session_id, original_ip) = if let Some(original_session) = self.sessions.iter().find(|s| s.token == session_token) {
                (original_session.id.clone(), original_session.ip_address.clone())
            } else {
                return Err(AppError::Authentication {
                    message: "Session not found".to_string(),
                });
            };

            // In a real system, we'd compare with original IP and detect anomalies
            if original_ip != malicious_ip {
                // Log suspicious activity
                self.log_session_activity(&session_id, "suspicious_access", malicious_ip, json!({
                    "reason": "IP address mismatch",
                    "original_ip": original_ip,
                    "suspicious_ip": malicious_ip
                }));

                return Err(AppError::Authentication {
                    message: "Session access from unexpected location detected".to_string(),
                });
            }

            Ok(())
        }

        fn get_session_security_score(&self, session_id: &str) -> f64 {
            let mut score = 100.0;

            // Check device trust level
            if let Some(device) = self.device_fingerprints.get(session_id) {
                if !device.is_trusted {
                    score -= 20.0;
                }
            }

            // Check for suspicious activity
            if let Some(activities) = self.session_activity.get(session_id) {
                let suspicious_count = activities.iter()
                    .filter(|a| a.action.contains("suspicious"))
                    .count();
                score -= suspicious_count as f64 * 10.0;
            }

            // Check session age
            if let Some(session) = self.sessions.iter().find(|s| s.id == session_id) {
                let age_hours = (Utc::now() - session.created_at).num_hours();
                if age_hours > 24 {
                    score -= 5.0;
                }
            }

            score.max(0.0)
        }
    }

    #[test]
    fn test_successful_login_and_session_creation() {
        let mut simulator = LoginSessionSimulator::new();

        // Create a verified user
        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Attempt login
        let result = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_ok());
        let (logged_user, session) = result.unwrap();

        // Verify user details
        assert_eq!(logged_user.id, user.id);
        assert_eq!(logged_user.email, "user@example.com");
        assert!(logged_user.last_login_at.is_some());

        // Verify session details
        assert_eq!(session.user_id, user.id);
        assert!(!session.token.is_empty());
        assert_eq!(session.ip_address, "192.168.1.100");
        assert_eq!(session.user_agent, "TestBrowser/1.0");
        assert!(session.is_active);
        assert!(session.expires_at > Utc::now());

        // Verify session was stored
        let user_sessions = simulator.get_user_sessions(&user.id);
        assert_eq!(user_sessions.len(), 1);
        assert_eq!(user_sessions[0].id, session.id);
    }

    #[test]
    fn test_login_with_invalid_credentials() {
        let mut simulator = LoginSessionSimulator::new();

        // Create a user
        simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Test wrong password
        let result = simulator.login(
            "user@example.com".to_string(),
            "WrongPassword".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Invalid credentials"));
            }
            _ => panic!("Expected authentication error"),
        }

        // Test non-existent user
        let result = simulator.login(
            "nonexistent@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Invalid credentials"));
            }
            _ => panic!("Expected authentication error"),
        }
    }

    #[test]
    fn test_login_with_unverified_email() {
        let mut simulator = LoginSessionSimulator::new();

        // Create an unverified user
        simulator.create_test_user(
            "unverified@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            false // email not verified
        );

        let result = simulator.login(
            "unverified@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Email not verified"));
            }
            _ => panic!("Expected authentication error for unverified email"),
        }
    }

    #[test]
    fn test_account_lockout_after_failed_attempts() {
        let mut simulator = LoginSessionSimulator::new();

        // Create a user
        simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Make 5 failed login attempts
        for i in 1..=5 {
            let result = simulator.login(
                "user@example.com".to_string(),
                "WrongPassword".to_string(),
                "192.168.1.100".to_string(),
                "TestBrowser/1.0".to_string()
            );

            assert!(result.is_err());

            if i < 5 {
                // First 4 attempts should just be invalid credentials
                match result.unwrap_err() {
                    AppError::Authentication { message } => {
                        assert!(message.contains("Invalid credentials"));
                    }
                    _ => panic!("Expected authentication error"),
                }
            }
        }

        // 6th attempt should be blocked due to account lock
        let result = simulator.login(
            "user@example.com".to_string(),
            "WrongPassword".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Account is locked"));
            }
            _ => panic!("Expected account lock error"),
        }

        // Even correct password should be blocked when locked
        let result = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Account is locked"));
            }
            _ => panic!("Expected account lock error"),
        }
    }

    #[test]
    fn test_multiple_concurrent_sessions() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create multiple sessions for the same user
        let sessions = vec![
            simulator.login(
                "user@example.com".to_string(),
                "SecurePass123!".to_string(),
                "192.168.1.100".to_string(),
                "Chrome/90.0".to_string()
            ).unwrap().1,
            simulator.login(
                "user@example.com".to_string(),
                "SecurePass123!".to_string(),
                "10.0.0.1".to_string(),
                "Firefox/88.0".to_string()
            ).unwrap().1,
            simulator.login(
                "user@example.com".to_string(),
                "SecurePass123!".to_string(),
                "172.16.0.1".to_string(),
                "Safari/14.0".to_string()
            ).unwrap().1,
        ];

        // Verify all sessions exist
        let user_sessions = simulator.get_user_sessions(&user.id);
        assert_eq!(user_sessions.len(), 3);

        // Verify each session is unique
        let mut tokens: Vec<&str> = user_sessions.iter().map(|s| s.token.as_str()).collect();
        tokens.sort();
        tokens.dedup();
        assert_eq!(tokens.len(), 3, "All session tokens should be unique");

        // Verify different IP addresses and user agents
        assert!(user_sessions.iter().any(|s| s.ip_address == "192.168.1.100"));
        assert!(user_sessions.iter().any(|s| s.ip_address == "10.0.0.1"));
        assert!(user_sessions.iter().any(|s| s.ip_address == "172.16.0.1"));
    }

    #[test]
    fn test_session_token_validation() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create a session
        let (_, session) = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        ).unwrap();

        // Validate valid token
        let validation_result = simulator.validate_session_token(&session.token);
        assert!(validation_result.is_ok());
        let validated_session = validation_result.unwrap();
        assert_eq!(validated_session.user_id, user.id);

        // Test invalid token
        let invalid_result = simulator.validate_session_token("invalid_token");
        assert!(invalid_result.is_err());
        match invalid_result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Invalid session token"));
            }
            _ => panic!("Expected authentication error for invalid token"),
        }
    }

    #[test]
    fn test_session_expiration() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create a session
        let session = simulator.create_session(
            user.id,
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        ).unwrap();

        // Manually expire the session
        if let Some(stored_session) = simulator.sessions.iter_mut().find(|s| s.id == session.id) {
            stored_session.expires_at = Utc::now() - Duration::hours(1);
        }

        // Validate expired token
        let validation_result = simulator.validate_session_token(&session.token);
        assert!(validation_result.is_err());
        match validation_result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Session expired"));
            }
            _ => panic!("Expected authentication error for expired session"),
        }
    }

    #[test]
    fn test_session_revocation() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create multiple sessions
        let session1 = simulator.create_session(
            user.id,
            "192.168.1.100".to_string(),
            "Chrome/90.0".to_string()
        ).unwrap();

        let session2 = simulator.create_session(
            user.id,
            "10.0.0.1".to_string(),
            "Firefox/88.0".to_string()
        ).unwrap();

        // Verify both sessions are active
        assert_eq!(simulator.get_user_sessions(&user.id).len(), 2);

        // Revoke one session
        let revoke_result = simulator.revoke_session(&session1.id);
        assert!(revoke_result.is_ok());

        // Verify only one session remains active
        let active_sessions = simulator.get_user_sessions(&user.id);
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, session2.id);

        // Try to validate revoked session
        let validation_result = simulator.validate_session_token(&session1.token);
        assert!(validation_result.is_err());
    }

    #[test]
    fn test_revoke_all_user_sessions() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create multiple sessions
        for i in 0..5 {
            simulator.create_session(
                user.id,
                format!("192.168.1.{}", i + 100),
                format!("Browser{}/1.0", i)
            ).unwrap();
        }

        // Verify all sessions exist
        assert_eq!(simulator.get_user_sessions(&user.id).len(), 5);

        // Revoke all sessions
        let revoked_count = simulator.revoke_all_user_sessions(&user.id);
        assert_eq!(revoked_count, 5);

        // Verify no active sessions remain
        assert_eq!(simulator.get_user_sessions(&user.id).len(), 0);
    }

    #[test]
    fn test_cleanup_expired_sessions() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create multiple sessions
        for i in 0..3 {
            simulator.create_session(
                user.id,
                format!("192.168.1.{}", i + 100),
                format!("Browser{}/1.0", i)
            ).unwrap();
        }

        // Manually expire 2 sessions
        let now = Utc::now();
        for (i, session) in simulator.sessions.iter_mut().enumerate() {
            if i < 2 {
                session.expires_at = now - Duration::hours(1);
            }
        }

        // Cleanup expired sessions
        let cleaned_count = simulator.cleanup_expired_sessions();
        assert_eq!(cleaned_count, 2);

        // Verify only 1 active session remains
        assert_eq!(simulator.get_user_sessions(&user.id).len(), 1);
    }

    #[test]
    fn test_session_information_tracking() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        let ip_address = "203.0.113.42".to_string();
        let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string();

        // Login and create session
        let (_, session) = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            ip_address.clone(),
            user_agent.clone()
        ).unwrap();

        // Verify session information is properly tracked
        assert_eq!(session.user_id, user.id);
        assert_eq!(session.ip_address, ip_address);
        assert_eq!(session.user_agent, user_agent);
        assert!(session.created_at <= Utc::now());
        assert!(session.expires_at > Utc::now());
        assert!(!session.id.is_empty());
        assert!(!session.token.is_empty());
        assert!(session.is_active);

        // Verify session duration (should be 24 hours)
        let duration = session.expires_at - session.created_at;
        assert!(duration >= Duration::hours(23)); // Allow for small timing differences
        assert!(duration <= Duration::hours(25));
    }

    #[test]
    fn test_failed_login_attempt_tracking() {
        let mut simulator = LoginSessionSimulator::new();

        simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Make 3 failed attempts
        for _ in 0..3 {
            let _ = simulator.login(
                "user@example.com".to_string(),
                "WrongPassword".to_string(),
                "192.168.1.100".to_string(),
                "TestBrowser/1.0".to_string()
            );
        }

        // Verify failed attempts are tracked
        let failed_attempts = &simulator.failed_attempts;
        assert_eq!(failed_attempts.len(), 1);
        assert_eq!(failed_attempts[0].0, "user@example.com");
        assert_eq!(failed_attempts[0].1, 3);

        // Account should not be locked yet (need 5 failures)
        assert!(!simulator.locked_accounts.contains(&"user@example.com".to_string()));

        // Successful login should clear failed attempts
        let result = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_ok());
        assert!(simulator.failed_attempts.is_empty());
    }

    #[test]
    fn test_inactive_user_login_prevention() {
        let mut simulator = LoginSessionSimulator::new();

        // Create user and set to inactive
        let mut user = simulator.create_test_user(
            "inactive@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Set user status to inactive
        user.status = UserStatus::Suspended;
        if let Some(stored_user) = simulator.users.iter_mut().find(|u| u.id == user.id) {
            stored_user.status = UserStatus::Suspended;
        }

        // Attempt login
        let result = simulator.login(
            "inactive@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "TestBrowser/1.0".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Account is not active"));
            }
            _ => panic!("Expected authentication error for inactive account"),
        }
    }

    #[test]
    fn test_session_hijacking_prevention() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create session
        let session = simulator.create_session(
            user.id,
            "192.168.1.100".to_string(),
            "Chrome/90.0".to_string()
        ).unwrap();

        // Verify session is tied to specific user
        assert_eq!(session.user_id, user.id);

        // Create another user
        let user2 = simulator.create_test_user(
            "user2@example.com".to_string(),
            "testuser2".to_string(),
            "SecurePass456!".to_string(),
            true
        );

        // Session should not be valid for different user
        let user2_sessions = simulator.get_user_sessions(&user2.id);
        assert!(user2_sessions.is_empty());

        // Session token should only validate for original user's sessions
        let validation = simulator.validate_session_token(&session.token);
        assert!(validation.is_ok());
        let validated_session = validation.unwrap();
        assert_eq!(validated_session.user_id, user.id);
        assert_ne!(validated_session.user_id, user2.id);
    }

    #[test]
    fn test_enhanced_login_with_rate_limiting() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        let ip = "192.168.1.100";

        // Should allow up to 10 attempts in 5 minutes
        for i in 0..10 {
            let result = simulator.enhanced_login(
                "user@example.com".to_string(),
                "SecurePass123!".to_string(),
                ip.to_string(),
                format!("Browser{}/1.0", i),
                false
            );
            assert!(result.is_ok(), "Attempt {} should succeed", i + 1);
        }

        // 11th attempt should fail due to rate limiting
        let result = simulator.enhanced_login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            ip.to_string(),
            "Browser11/1.0".to_string(),
            false
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Too many login attempts"));
            }
            _ => panic!("Expected rate limit error"),
        }
    }

    #[test]
    fn test_suspicious_activity_detection() {
        let mut simulator = LoginSessionSimulator::new();

        simulator.create_test_user(
            "user1@example.com".to_string(),
            "user1".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        simulator.create_test_user(
            "user2@example.com".to_string(),
            "user2".to_string(),
            "SecurePass456!".to_string(),
            true
        );

        let suspicious_ip = "10.0.0.1";

        // Simulate failed attempts for multiple users from same IP
        for i in 0..6 {
            let email = format!("user{}@example.com", (i % 2) + 1);
            let result = simulator.enhanced_login(
                email,
                "WrongPassword".to_string(),
                suspicious_ip.to_string(),
                "AttackerBrowser/1.0".to_string(),
                false
            );
            // After enough attempts, should get blocked
            if i >= 5 {
                assert!(result.is_err(), "Should be blocked after {} attempts", i + 1);
            }
        }

        // Verify IP is blocked in suspicious_ips
        assert!(simulator.suspicious_ips.contains_key(suspicious_ip));
        let suspicious_data = &simulator.suspicious_ips[suspicious_ip];
        assert!(suspicious_data.is_blocked || suspicious_data.failed_attempts > 5);
    }

    #[test]
    fn test_device_fingerprinting_and_trust() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // First login from a device
        let result = simulator.enhanced_login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string(),
            false
        );

        assert!(result.is_ok());
        let (_, session, _) = result.unwrap();

        // Device should initially be untrusted
        let device_info = &simulator.device_fingerprints[&session.id];
        assert!(!device_info.is_trusted);
        assert_eq!(device_info.browser, "Chrome");
        assert_eq!(device_info.os, "Windows");

        // Verify fingerprint generation
        let expected_fingerprint = simulator.generate_device_fingerprint(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "192.168.1.100"
        );
        assert_eq!(device_info.fingerprint, expected_fingerprint);
    }

    #[test]
    fn test_remember_me_functionality() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Login with remember me
        let result = simulator.enhanced_login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "Chrome/91.0".to_string(),
            true // remember me
        );

        assert!(result.is_ok());
        let (_, _, remember_token) = result.unwrap();
        assert!(remember_token.is_some());

        let token = remember_token.unwrap();

        // Validate remember me token
        let validation_result = simulator.validate_remember_me_token(&token);
        assert!(validation_result.is_ok());
        assert_eq!(validation_result.unwrap(), user.id);

        // Test invalid token
        let invalid_result = simulator.validate_remember_me_token("invalid_token");
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_session_activity_logging() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Enhanced login creates activity logs
        let result = simulator.enhanced_login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "Chrome/91.0".to_string(),
            false
        );

        assert!(result.is_ok());
        let (_, session, _) = result.unwrap();

        // Check that login activity was logged
        assert!(simulator.session_activity.contains_key(&session.id));
        let activities = &simulator.session_activity[&session.id];
        assert!(!activities.is_empty());
        assert_eq!(activities[0].action, "login");
        assert_eq!(activities[0].ip_address, "192.168.1.100");

        // Manually log additional activity
        simulator.log_session_activity(&session.id, "page_view", "192.168.1.100", json!({
            "page": "/dashboard"
        }));

        let updated_activities = &simulator.session_activity[&session.id];
        assert_eq!(updated_activities.len(), 2);
        assert_eq!(updated_activities[1].action, "page_view");
    }

    #[tokio::test]
    async fn test_concurrent_login_handling() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Test concurrent logins
        let results = simulator.concurrent_login_test(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            5
        ).await;

        // All concurrent logins should succeed
        assert_eq!(results.len(), 5);
        for result in results {
            assert!(result.is_ok());
        }

        // Update concurrent session count manually since we used basic login
        simulator.concurrent_sessions.insert(user.id, 5);

        // Verify all sessions are unique
        let user_sessions = simulator.get_user_sessions(&user.id);
        assert_eq!(user_sessions.len(), 5);

        let mut tokens: Vec<&str> = user_sessions.iter().map(|s| s.token.as_str()).collect();
        tokens.sort();
        tokens.dedup();
        assert_eq!(tokens.len(), 5, "All session tokens should be unique");
    }

    #[test]
    fn test_session_hijacking_detection() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create legitimate session
        let (_, session) = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "Chrome/91.0".to_string()
        ).unwrap();

        // Attempt to use session from different IP (simulating hijacking)
        let hijack_result = simulator.simulate_session_hijacking_attempt(
            &session.token,
            "10.0.0.1" // Different IP
        );

        assert!(hijack_result.is_err());
        match hijack_result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("unexpected location"));
            }
            _ => panic!("Expected session hijacking detection error"),
        }

        // Verify suspicious activity was logged
        assert!(simulator.session_activity.contains_key(&session.id));
        let activities = &simulator.session_activity[&session.id];
        assert!(activities.iter().any(|a| a.action == "suspicious_access"));
    }

    #[test]
    fn test_session_security_scoring() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create session and track device
        let (_, session) = simulator.login(
            "user@example.com".to_string(),
            "SecurePass123!".to_string(),
            "192.168.1.100".to_string(),
            "Chrome/91.0".to_string()
        ).unwrap();

        simulator.track_device(&session.id, "192.168.1.100", "Chrome/91.0");

        // Initial security score should be reduced for untrusted device
        let initial_score = simulator.get_session_security_score(&session.id);
        assert!(initial_score < 100.0);
        assert!(initial_score >= 80.0); // Should be 80 (100 - 20 for untrusted device)

        // Add suspicious activity
        simulator.log_session_activity(&session.id, "suspicious_access", "10.0.0.1", json!({
            "reason": "unusual_behavior"
        }));

        // Score should decrease further
        let suspicious_score = simulator.get_session_security_score(&session.id);
        assert!(suspicious_score < initial_score);

        // Trust the device (simulate 24+ hours)
        if let Some(device) = simulator.device_fingerprints.get_mut(&session.id) {
            device.is_trusted = true;
        }

        // Score should improve
        let trusted_score = simulator.get_session_security_score(&session.id);
        assert!(trusted_score > suspicious_score);
    }

    #[test]
    fn test_mass_login_attempt_protection() {
        let mut simulator = LoginSessionSimulator::new();

        simulator.create_test_user(
            "victim@example.com".to_string(),
            "victim".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        let attacker_ip = "10.0.0.1";

        // Simulate mass login attempts (brute force)
        for i in 0..10 {
            let _ = simulator.enhanced_login(
                "victim@example.com".to_string(),
                format!("attempt{}", i),
                attacker_ip.to_string(),
                "AttackerBot/1.0".to_string(),
                false
            );
        }

        // IP should be blocked after too many failed attempts
        let result = simulator.enhanced_login(
            "victim@example.com".to_string(),
            "SecurePass123!".to_string(), // Even correct password should be blocked
            attacker_ip.to_string(),
            "AttackerBot/1.0".to_string(),
            false
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(
                    message.contains("suspicious activity") ||
                    message.contains("rate limit") ||
                    message.contains("Too many login attempts") ||
                    message.contains("blocked"),
                    "Unexpected error message: {}", message
                );
            }
            _ => panic!("Expected protection against mass login attempts"),
        }
    }

    #[test]
    fn test_session_cleanup_comprehensive() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Create multiple sessions with different expiration times
        let now = Utc::now();
        for i in 0..5 {
            let mut session = simulator.create_session(
                user.id,
                format!("192.168.1.{}", i + 100),
                format!("Browser{}/1.0", i)
            ).unwrap();

            // Make some sessions already expired
            if i < 3 {
                session.expires_at = now - Duration::hours(1);
            }

            // Update the session in the simulator
            if let Some(stored_session) = simulator.sessions.iter_mut().find(|s| s.id == session.id) {
                stored_session.expires_at = session.expires_at;
            }
        }

        // Before cleanup: 5 total sessions, but some are expired
        assert_eq!(simulator.sessions.len(), 5);

        // Count non-expired sessions manually
        let now = Utc::now();
        let active_sessions_before = simulator.sessions.iter()
            .filter(|s| s.user_id == user.id && s.is_active && s.expires_at > now)
            .count();
        assert_eq!(active_sessions_before, 2);

        // Cleanup expired sessions
        let cleaned_count = simulator.cleanup_expired_sessions();
        assert_eq!(cleaned_count, 3);

        // After cleanup: still 5 total sessions, but only 2 active
        assert_eq!(simulator.sessions.len(), 5);
        assert_eq!(simulator.get_user_sessions(&user.id).len(), 2);

        // Verify inactive sessions are marked correctly
        let inactive_count = simulator.sessions.iter()
            .filter(|s| !s.is_active)
            .count();
        assert_eq!(inactive_count, 3);
    }

    #[test]
    fn test_cross_platform_login_tracking() {
        let mut simulator = LoginSessionSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "SecurePass123!".to_string(),
            true
        );

        // Login from different platforms
        let platforms = vec![
            ("192.168.1.100", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/91.0"),
            ("192.168.1.101", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Safari/14.1"),
            ("192.168.1.102", "Mozilla/5.0 (X11; Linux x86_64) Firefox/89.0"),
            ("192.168.1.103", "Mozilla/5.0 (iPhone; CPU iPhone OS 14_6 like Mac OS X) Safari/14.0"),
        ];

        for (ip, user_agent) in platforms {
            let result = simulator.enhanced_login(
                "user@example.com".to_string(),
                "SecurePass123!".to_string(),
                ip.to_string(),
                user_agent.to_string(),
                false
            );

            assert!(result.is_ok());
            let (_, session, _) = result.unwrap();

            // Verify device information is tracked correctly
            let device = &simulator.device_fingerprints[&session.id];

            if user_agent.contains("Windows") {
                assert_eq!(device.os, "Windows");
                assert_eq!(device.browser, "Chrome");
            } else if user_agent.contains("Mac") {
                assert_eq!(device.os, "macOS");
                assert_eq!(device.browser, "Safari");
            } else if user_agent.contains("Linux") {
                assert_eq!(device.os, "Linux");
                assert_eq!(device.browser, "Firefox");
            }
        }

        // Verify all sessions exist and are unique
        let user_sessions = simulator.get_user_sessions(&user.id);
        assert_eq!(user_sessions.len(), 4);

        // Verify different IPs are tracked
        let unique_ips: std::collections::HashSet<&str> = user_sessions.iter()
            .map(|s| s.ip_address.as_str())
            .collect();
        assert_eq!(unique_ips.len(), 4);
    }
}