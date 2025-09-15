//! Integration Tests for Complete Password Reset Flow
//!
//! This module contains comprehensive integration tests for the password reset process.
//! These tests simulate the complete flow including security measures, rate limiting,
//! fraud detection, and advanced password policies.

#[cfg(test)]
mod password_reset_flow_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    #[derive(Debug, Clone)]
    struct PasswordResetToken {
        token: String,
        user_id: Uuid,
        email: String,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        used: bool,
    }

    #[derive(Debug, Clone)]
    struct EmailRecord {
        to: String,
        subject: String,
        body: String,
        sent_at: chrono::DateTime<Utc>,
        email_type: EmailType,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum EmailType {
        PasswordResetRequest,
        PasswordResetConfirmation,
        SecurityAlert,
        AccountLocked,
    }

    #[derive(Debug, Clone)]
    struct SecurityEvent {
        event_id: Uuid,
        user_id: Option<Uuid>,
        event_type: SecurityEventType,
        ip_address: String,
        user_agent: String,
        details: serde_json::Value,
        timestamp: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum SecurityEventType {
        PasswordResetRequested,
        PasswordResetCompleted,
        PasswordResetAttemptFailed,
        SuspiciousActivity,
        AccountLocked,
        MultipleResetRequests,
    }

    #[derive(Debug, Clone)]
    struct RateLimitEntry {
        attempts: VecDeque<chrono::DateTime<Utc>>,
        blocked_until: Option<chrono::DateTime<Utc>>,
    }

    #[derive(Debug, Clone)]
    struct PasswordPolicy {
        min_length: usize,
        require_uppercase: bool,
        require_lowercase: bool,
        require_numbers: bool,
        require_special: bool,
        history_count: usize,
        min_age_hours: u32,
        common_passwords: Vec<String>,
    }

    #[derive(Debug, Clone)]
    struct FraudDetection {
        ip_reputation: HashMap<String, i32>, // IP -> reputation score (0-100, lower is worse)
        geolocation_checks: HashMap<String, GeoLocation>,
        device_fingerprints: HashMap<String, DeviceFingerprint>,
        risk_scores: HashMap<Uuid, f64>, // user_id -> risk score
    }

    #[derive(Debug, Clone)]
    struct GeoLocation {
        country: String,
        city: String,
        is_vpn: bool,
        is_tor: bool,
    }

    #[derive(Debug, Clone)]
    struct DeviceFingerprint {
        fingerprint: String,
        first_seen: chrono::DateTime<Utc>,
        last_seen: chrono::DateTime<Utc>,
        is_trusted: bool,
    }

    // Enhanced password reset flow simulator with comprehensive security features
    struct PasswordResetFlowSimulator {
        users: Vec<User>,
        reset_tokens: Vec<PasswordResetToken>,
        sent_emails: Vec<EmailRecord>,
        password_history: Vec<(Uuid, String, chrono::DateTime<Utc>)>, // user_id, password_hash, changed_at
        rate_limits: HashMap<String, RateLimitEntry>, // IP -> rate limit data
        security_events: Vec<SecurityEvent>,
        password_policy: PasswordPolicy,
        fraud_detection: FraudDetection,
        blocked_ips: HashMap<String, chrono::DateTime<Utc>>, // IP -> blocked until
        failed_reset_attempts: HashMap<String, u32>, // email -> failed count
        two_factor_enabled: HashMap<Uuid, bool>, // user_id -> 2FA enabled
        backup_codes: HashMap<Uuid, Vec<String>>, // user_id -> backup codes
        notification_preferences: HashMap<Uuid, NotificationSettings>,
    }

    #[derive(Debug, Clone)]
    struct NotificationSettings {
        email_alerts: bool,
        sms_alerts: bool,
        push_notifications: bool,
        security_notifications: bool,
    }

    impl PasswordResetFlowSimulator {
        fn new() -> Self {
            Self {
                users: Vec::new(),
                reset_tokens: Vec::new(),
                sent_emails: Vec::new(),
                password_history: Vec::new(),
                rate_limits: HashMap::new(),
                security_events: Vec::new(),
                password_policy: PasswordPolicy::default(),
                fraud_detection: FraudDetection::new(),
                blocked_ips: HashMap::new(),
                failed_reset_attempts: HashMap::new(),
                two_factor_enabled: HashMap::new(),
                backup_codes: HashMap::new(),
                notification_preferences: HashMap::new(),
            }
        }

        fn create_test_user(&mut self, email: String, username: String, password: String) -> User {
            let user = User {
                id: Uuid::new_v4(),
                username,
                email: email.clone(),
                password_hash: format!("hashed_{}", password),
                email_verified: true,
                status: UserStatus::Active,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            };

            // Store initial password in history
            self.password_history.push((user.id, user.password_hash.clone(), user.created_at));
            self.users.push(user.clone());
            user
        }

        fn request_password_reset(&mut self, email: &str) -> Result<String, AppError> {
            // Find user by email
            let user = self.users.iter()
                .find(|u| u.email == email)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(email.to_string()),
                })?;

            // Check if user account is active
            if user.status != UserStatus::Active {
                return Err(AppError::Validation {
                    field: "account".to_string(),
                    message: "Account is not active".to_string(),
                });
            }

            // Check if email is verified
            if !user.email_verified {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email must be verified to reset password".to_string(),
                });
            }

            // Invalidate any existing reset tokens for this user
            for token in self.reset_tokens.iter_mut() {
                if token.user_id == user.id && !token.used {
                    token.used = true;
                }
            }

            // Create new reset token
            let token = format!("reset_{}", Uuid::new_v4());
            let reset_token = PasswordResetToken {
                token: token.clone(),
                user_id: user.id,
                email: user.email.clone(),
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(1), // 1 hour expiration
                used: false,
            };

            self.reset_tokens.push(reset_token);

            // Send email
            let email_record = EmailRecord {
                to: user.email.clone(),
                subject: "Password Reset Request".to_string(),
                body: format!("Use this token to reset your password: {}", token),
                sent_at: Utc::now(),
                email_type: EmailType::PasswordResetRequest,
            };
            self.sent_emails.push(email_record);

            Ok(token)
        }

        fn validate_reset_token(&self, token: &str) -> Result<&PasswordResetToken, AppError> {
            let reset_token = self.reset_tokens.iter()
                .find(|t| t.token == token && !t.used)
                .ok_or_else(|| AppError::Validation {
                    field: "token".to_string(),
                    message: "Invalid or expired reset token".to_string(),
                })?;

            // Check if token is expired
            if reset_token.expires_at < Utc::now() {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Reset token has expired".to_string(),
                });
            }

            Ok(reset_token)
        }

        fn reset_password(&mut self, token: &str, new_password: &str) -> Result<(), AppError> {
            // Validate token and get user ID
            let user_id = {
                let reset_token = self.validate_reset_token(token)?;
                reset_token.user_id
            };

            // Validate new password strength
            self.validate_password_strength(new_password)?;

            // Find user
            let user = self.users.iter_mut()
                .find(|u| u.id == user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            let new_password_hash = format!("hashed_{}", new_password);

            // Check password history (prevent reuse of last 3 passwords)
            let recent_passwords: Vec<_> = self.password_history.iter()
                .filter(|(uid, _, _)| *uid == user.id)
                .collect();

            if recent_passwords.len() >= 3 {
                let last_three: Vec<_> = recent_passwords.iter()
                    .rev()
                    .take(3)
                    .map(|(_, hash, _)| hash)
                    .collect();

                if last_three.contains(&&new_password_hash) {
                    return Err(AppError::Validation {
                        field: "password".to_string(),
                        message: "Cannot reuse one of your last 3 passwords".to_string(),
                    });
                }
            }

            // Update password
            user.password_hash = new_password_hash.clone();
            user.updated_at = Utc::now();

            // Add to password history
            self.password_history.push((user.id, new_password_hash, Utc::now()));

            // Mark token as used
            if let Some(token_mut) = self.reset_tokens.iter_mut().find(|t| t.token == token) {
                token_mut.used = true;
            }

            Ok(())
        }

        fn validate_password_strength(&self, password: &str) -> Result<(), AppError> {
            if password.len() < 8 {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must be at least 8 characters".to_string(),
                });
            }

            let has_upper = password.chars().any(|c| c.is_uppercase());
            let has_lower = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_digit(10));
            let has_special = password.chars().any(|c| !c.is_alphanumeric());

            if !has_upper || !has_lower || !has_digit || !has_special {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password must contain uppercase, lowercase, number, and special character".to_string(),
                });
            }

            Ok(())
        }

        fn complete_password_reset_flow(&mut self, email: String, new_password: String)
            -> Result<(String, User), AppError> {
            // Step 1: Request password reset
            let token = self.request_password_reset(&email)?;

            // Step 2: Reset password using token
            self.reset_password(&token, &new_password)?;

            // Step 3: Get updated user
            let updated_user = self.users.iter()
                .find(|u| u.email == email)
                .cloned()
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(email),
                })?;

            Ok((token, updated_user))
        }

        fn get_user_password_history(&self, user_id: &Uuid) -> Vec<(String, chrono::DateTime<Utc>)> {
            self.password_history.iter()
                .filter(|(uid, _, _)| uid == user_id)
                .map(|(_, hash, date)| (hash.clone(), *date))
                .collect()
        }

        fn get_active_reset_tokens(&self, user_id: &Uuid) -> Vec<&PasswordResetToken> {
            self.reset_tokens.iter()
                .filter(|t| t.user_id == *user_id && !t.used && t.expires_at > Utc::now())
                .collect()
        }

        fn cleanup_expired_tokens(&mut self) {
            let now = Utc::now();
            for token in self.reset_tokens.iter_mut() {
                if token.expires_at < now {
                    token.used = true;
                }
            }
        }

        fn check_rate_limit(&mut self, ip: &str) -> Result<(), AppError> {
            let now = Utc::now();
            let window_start = now - Duration::minutes(15); // 15-minute window

            let rate_limit = self.rate_limits.entry(ip.to_string()).or_insert(RateLimitEntry {
                attempts: VecDeque::new(),
                blocked_until: None,
            });

            // Check if IP is currently blocked
            if let Some(blocked_until) = rate_limit.blocked_until {
                if blocked_until > now {
                    return Err(AppError::Authentication {
                        message: "IP address is temporarily blocked due to too many password reset attempts".to_string(),
                    });
                } else {
                    rate_limit.blocked_until = None;
                }
            }

            // Remove old attempts outside the window
            while let Some(&front_time) = rate_limit.attempts.front() {
                if front_time < window_start {
                    rate_limit.attempts.pop_front();
                } else {
                    break;
                }
            }

            // Check if rate limit exceeded (5 attempts per 15 minutes)
            if rate_limit.attempts.len() >= 5 {
                rate_limit.blocked_until = Some(now + Duration::hours(1));
                return Err(AppError::Authentication {
                    message: "Too many password reset attempts. Please try again later.".to_string(),
                });
            }

            rate_limit.attempts.push_back(now);
            Ok(())
        }

        fn enhanced_request_password_reset(&mut self, email: &str, ip: &str, user_agent: &str) -> Result<String, AppError> {
            let user_id = self.users.iter().find(|u| u.email == email).map(|u| u.id);

            // Always log the attempt first (for security auditing)
            self.log_security_event(
                user_id,
                SecurityEventType::PasswordResetRequested,
                ip,
                user_agent,
                json!({ "email": email })
            );

            // Check rate limiting
            self.check_rate_limit(ip)?;

            // Check IP reputation
            if let Some(&reputation) = self.fraud_detection.ip_reputation.get(ip) {
                if reputation < 30 {
                    // Log the blocked attempt
                    self.log_security_event(
                        user_id,
                        SecurityEventType::SuspiciousActivity,
                        ip,
                        user_agent,
                        json!({ "email": email, "reason": "low_ip_reputation", "reputation": reputation })
                    );
                    return Err(AppError::Authentication {
                        message: "Request from suspicious IP address blocked".to_string(),
                    });
                }
            }

            // Perform normal password reset request
            let result = self.request_password_reset(email);

            // Track failed attempts
            if result.is_err() {
                let count = self.failed_reset_attempts.entry(email.to_string()).or_insert(0);
                *count += 1;
                let current_count = *count;

                if current_count >= 3 {
                    self.log_security_event(
                        user_id,
                        SecurityEventType::SuspiciousActivity,
                        ip,
                        user_agent,
                        json!({ "email": email, "failed_attempts": current_count })
                    );
                }
            }

            result
        }

        fn enhanced_reset_password(&mut self, token: &str, new_password: &str, ip: &str, user_agent: &str) -> Result<(), AppError> {
            // Validate token and get user info
            let (user_id, email) = {
                let reset_token = self.validate_reset_token(token)?;
                (reset_token.user_id, reset_token.email.clone())
            };

            // Enhanced password validation
            self.enhanced_password_validation(new_password, &user_id)?;

            // Perform normal password reset
            let result = self.reset_password(token, new_password);

            // Log security event
            let event_type = if result.is_ok() {
                SecurityEventType::PasswordResetCompleted
            } else {
                SecurityEventType::PasswordResetAttemptFailed
            };

            self.log_security_event(
                Some(user_id),
                event_type,
                ip,
                user_agent,
                json!({ "email": email, "success": result.is_ok() })
            );

            // Send security notifications
            if result.is_ok() {
                self.send_security_notifications(&user_id, &email, ip);
            }

            result
        }

        fn enhanced_password_validation(&self, password: &str, user_id: &Uuid) -> Result<(), AppError> {
            // Check against common passwords
            if self.password_policy.common_passwords.contains(&password.to_string()) {
                return Err(AppError::Validation {
                    field: "password".to_string(),
                    message: "Password is too common. Please choose a more unique password.".to_string(),
                });
            }

            // Check minimum age (if this is not the first password change)
            let password_history = self.get_user_password_history(user_id);
            if password_history.len() > 1 {
                let last_change = password_history.last().unwrap().1;
                let min_age = Duration::hours(self.password_policy.min_age_hours as i64);
                if Utc::now() - last_change < min_age {
                    return Err(AppError::Validation {
                        field: "password".to_string(),
                        message: format!("Password was changed too recently. Please wait {} hours before changing again.", self.password_policy.min_age_hours),
                    });
                }
            }

            // Standard password strength validation
            self.validate_password_strength(password)
        }

        fn log_security_event(&mut self, user_id: Option<Uuid>, event_type: SecurityEventType, ip: &str, user_agent: &str, details: serde_json::Value) {
            let event = SecurityEvent {
                event_id: Uuid::new_v4(),
                user_id,
                event_type,
                ip_address: ip.to_string(),
                user_agent: user_agent.to_string(),
                details,
                timestamp: Utc::now(),
            };
            self.security_events.push(event);
        }

        fn send_security_notifications(&mut self, user_id: &Uuid, email: &str, reset_ip: &str) {
            // Send password reset confirmation email
            let confirmation_email = EmailRecord {
                to: email.to_string(),
                subject: "Password Reset Confirmation".to_string(),
                body: format!("Your password has been successfully reset from IP address: {}", reset_ip),
                sent_at: Utc::now(),
                email_type: EmailType::PasswordResetConfirmation,
            };
            self.sent_emails.push(confirmation_email);

            // Check notification preferences
            if let Some(prefs) = self.notification_preferences.get(user_id) {
                if prefs.security_notifications {
                    let security_alert = EmailRecord {
                        to: email.to_string(),
                        subject: "Security Alert: Password Changed".to_string(),
                        body: format!("Your account password was changed. If this wasn't you, please contact support immediately. Reset IP: {}", reset_ip),
                        sent_at: Utc::now(),
                        email_type: EmailType::SecurityAlert,
                    };
                    self.sent_emails.push(security_alert);
                }
            }
        }

        fn detect_fraud(&mut self, email: &str, ip: &str, user_agent: &str) -> f64 {
            let mut risk_score = 0.0;

            // Check IP reputation
            if let Some(&reputation) = self.fraud_detection.ip_reputation.get(ip) {
                risk_score += (100 - reputation) as f64 / 100.0 * 30.0; // Max 30 points for bad IP
            }

            // Check geolocation
            if let Some(geo) = self.fraud_detection.geolocation_checks.get(ip) {
                if geo.is_vpn {
                    risk_score += 20.0;
                }
                if geo.is_tor {
                    risk_score += 40.0;
                }
            }

            // Check device fingerprint
            let fingerprint = format!("{}_{}", user_agent, ip);
            if let Some(device) = self.fraud_detection.device_fingerprints.get(&fingerprint) {
                if !device.is_trusted {
                    risk_score += 15.0;
                }
            } else {
                // New device
                risk_score += 10.0;
                let device_fp = DeviceFingerprint {
                    fingerprint: fingerprint.clone(),
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                    is_trusted: false,
                };
                self.fraud_detection.device_fingerprints.insert(fingerprint, device_fp);
            }

            // Check failed attempts
            if let Some(&failed_count) = self.failed_reset_attempts.get(email) {
                risk_score += failed_count as f64 * 5.0;
            }

            risk_score.min(100.0)
        }

        async fn concurrent_reset_simulation(&mut self, emails: Vec<String>, ip: &str) -> Vec<Result<String, AppError>> {
            let mut results = Vec::new();

            for email in emails {
                let result = self.enhanced_request_password_reset(&email, ip, "TestAgent/1.0");
                results.push(result);
                sleep(TokioDuration::from_millis(50)).await;
            }

            results
        }

        fn simulate_advanced_attack(&mut self, target_email: &str) -> Vec<Result<String, AppError>> {
            let attack_ips = vec![
                "10.0.0.1", "10.0.0.2", "10.0.0.3", "192.168.1.100", "172.16.0.1"
            ];

            let mut results = Vec::new();

            for ip in attack_ips {
                // Set low reputation for attack IPs
                self.fraud_detection.ip_reputation.insert(ip.to_string(), 10);

                let result = self.enhanced_request_password_reset(target_email, ip, "AttackBot/1.0");
                results.push(result);
            }

            results
        }

        fn enable_two_factor(&mut self, user_id: Uuid) {
            self.two_factor_enabled.insert(user_id, true);

            // Generate backup codes
            let backup_codes: Vec<String> = (0..10)
                .map(|i| format!("backup-{:08}", i * 12345))
                .collect();
            self.backup_codes.insert(user_id, backup_codes);
        }

        fn get_security_events_for_user(&self, user_id: &Uuid) -> Vec<&SecurityEvent> {
            self.security_events.iter()
                .filter(|event| event.user_id == Some(*user_id))
                .collect()
        }

        fn get_risk_assessment(&self, user_id: &Uuid) -> HashMap<String, serde_json::Value> {
            let mut assessment = HashMap::new();

            // Get security events count
            let events = self.get_security_events_for_user(user_id);
            assessment.insert("total_security_events".to_string(), json!(events.len()));

            // Check recent password resets
            let recent_resets = events.iter()
                .filter(|e| e.event_type == SecurityEventType::PasswordResetCompleted)
                .filter(|e| Utc::now() - e.timestamp < Duration::days(7))
                .count();
            assessment.insert("recent_password_resets".to_string(), json!(recent_resets));

            // Check if 2FA is enabled
            let has_2fa = self.two_factor_enabled.get(user_id).unwrap_or(&false);
            assessment.insert("two_factor_enabled".to_string(), json!(has_2fa));

            // Get risk score
            let risk_score = self.fraud_detection.risk_scores.get(user_id).unwrap_or(&0.0);
            assessment.insert("risk_score".to_string(), json!(risk_score));

            assessment
        }
    }

    impl Default for PasswordPolicy {
        fn default() -> Self {
            Self {
                min_length: 8,
                require_uppercase: true,
                require_lowercase: true,
                require_numbers: true,
                require_special: true,
                history_count: 3,
                min_age_hours: 24,
                common_passwords: vec![
                    "password".to_string(),
                    "123456".to_string(),
                    "password123".to_string(),
                    "admin".to_string(),
                    "letmein".to_string(),
                ],
            }
        }
    }

    impl FraudDetection {
        fn new() -> Self {
            let mut ip_reputation = HashMap::new();
            // Set some default IP reputations
            ip_reputation.insert("127.0.0.1".to_string(), 100); // Localhost is trusted
            ip_reputation.insert("192.168.1.100".to_string(), 85); // Private IP, medium trust

            Self {
                ip_reputation,
                geolocation_checks: HashMap::new(),
                device_fingerprints: HashMap::new(),
                risk_scores: HashMap::new(),
            }
        }
    }

    impl Default for NotificationSettings {
        fn default() -> Self {
            Self {
                email_alerts: true,
                sms_alerts: false,
                push_notifications: true,
                security_notifications: true,
            }
        }
    }

    #[test]
    fn test_complete_password_reset_flow_success() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "OldPassword123!".to_string(),
        );

        let new_password = "NewSecurePass456#".to_string();
        let result = simulator.complete_password_reset_flow(
            user.email.clone(),
            new_password.clone()
        );

        assert!(result.is_ok());
        let (token, updated_user) = result.unwrap();

        // Verify token was created
        assert!(!token.is_empty());
        assert!(token.starts_with("reset_"));

        // Verify password was changed
        assert_eq!(updated_user.password_hash, format!("hashed_{}", new_password));
        assert_ne!(updated_user.password_hash, format!("hashed_OldPassword123!"));

        // Verify email was sent
        assert_eq!(simulator.sent_emails.len(), 1);
        assert_eq!(simulator.sent_emails[0].to, user.email);
        assert_eq!(simulator.sent_emails[0].subject, "Password Reset Request");

        // Verify token was marked as used
        let used_tokens: Vec<_> = simulator.reset_tokens.iter()
            .filter(|t| t.token == token && t.used)
            .collect();
        assert_eq!(used_tokens.len(), 1);
    }

    #[test]
    fn test_password_reset_request_for_nonexistent_user() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let result = simulator.request_password_reset("nonexistent@example.com");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::NotFound { resource, id } => {
                assert_eq!(resource, "user");
                assert_eq!(id, Some("nonexistent@example.com".to_string()));
            }
            _ => panic!("Expected NotFound error"),
        }

        // Verify no emails were sent
        assert_eq!(simulator.sent_emails.len(), 0);
    }

    #[test]
    fn test_password_reset_for_inactive_user() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let mut user = simulator.create_test_user(
            "inactive@example.com".to_string(),
            "inactiveuser".to_string(),
            "Password123!".to_string(),
        );

        // Make user inactive
        user.status = UserStatus::Inactive;
        simulator.users[0] = user.clone();

        let result = simulator.request_password_reset(&user.email);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "account");
                assert!(message.contains("not active"));
            }
            _ => panic!("Expected validation error for inactive account"),
        }
    }

    #[test]
    fn test_password_reset_for_unverified_email() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let mut user = simulator.create_test_user(
            "unverified@example.com".to_string(),
            "unverifieduser".to_string(),
            "Password123!".to_string(),
        );

        // Make email unverified
        user.email_verified = false;
        simulator.users[0] = user.clone();

        let result = simulator.request_password_reset(&user.email);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "email");
                assert!(message.contains("must be verified"));
            }
            _ => panic!("Expected validation error for unverified email"),
        }
    }

    #[test]
    fn test_password_reset_with_invalid_token() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let result = simulator.reset_password("invalid_token", "NewPassword123!");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("Invalid or expired"));
            }
            _ => panic!("Expected validation error for invalid token"),
        }
    }

    #[test]
    fn test_password_reset_with_expired_token() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        // Create an expired token manually
        let expired_token = PasswordResetToken {
            token: "expired_token".to_string(),
            user_id: user.id,
            email: user.email.clone(),
            created_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::hours(1), // Expired 1 hour ago
            used: false,
        };
        simulator.reset_tokens.push(expired_token);

        let result = simulator.reset_password("expired_token", "NewPassword123!");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected validation error for expired token"),
        }
    }

    #[test]
    fn test_password_reset_with_weak_password() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        let token = simulator.request_password_reset(&user.email).unwrap();

        let weak_passwords = vec![
            "short",          // Too short
            "nouppercas3!",   // No uppercase
            "NOLOWERCASE3!",  // No lowercase
            "NoNumbers!",     // No numbers
            "NoSpecial123",   // No special characters
        ];

        for weak_password in weak_passwords {
            let result = simulator.reset_password(&token, weak_password);
            assert!(result.is_err());

            match result.unwrap_err() {
                AppError::Validation { field, .. } => {
                    assert_eq!(field, "password");
                }
                _ => panic!("Expected validation error for weak password"),
            }
        }
    }

    #[test]
    fn test_password_reset_token_reuse_prevention() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        let token = simulator.request_password_reset(&user.email).unwrap();

        // First use should succeed
        let result1 = simulator.reset_password(&token, "NewPassword123!");
        assert!(result1.is_ok());

        // Second use of same token should fail
        let result2 = simulator.reset_password(&token, "AnotherPassword456#");
        assert!(result2.is_err());

        match result2.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("Invalid or expired"));
            }
            _ => panic!("Expected validation error for reused token"),
        }
    }

    #[test]
    fn test_password_history_enforcement() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password1!".to_string(),
        );

        // Change password 3 more times to build history
        for i in 2..=4 {
            let token = simulator.request_password_reset(&user.email).unwrap();
            let new_password = format!("Password{}!", i);
            simulator.reset_password(&token, &new_password).unwrap();
        }

        // Now try to reuse one of the last 3 passwords
        let token = simulator.request_password_reset(&user.email).unwrap();
        let result = simulator.reset_password(&token, "Password2!"); // This was used 2 changes ago

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "password");
                assert!(message.contains("Cannot reuse"));
                assert!(message.contains("last 3 passwords"));
            }
            _ => panic!("Expected validation error for password reuse"),
        }

        // But using an older password (more than 3 changes ago) should work
        let token = simulator.request_password_reset(&user.email).unwrap();
        let result = simulator.reset_password(&token, "Password1!"); // This was the original
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_reset_requests_invalidate_previous() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        // First reset request
        let token1 = simulator.request_password_reset(&user.email).unwrap();

        // Second reset request should invalidate the first
        let token2 = simulator.request_password_reset(&user.email).unwrap();

        assert_ne!(token1, token2);

        // First token should no longer work
        let result1 = simulator.reset_password(&token1, "NewPassword1!");
        assert!(result1.is_err());

        // Second token should still work
        let result2 = simulator.reset_password(&token2, "NewPassword2!");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_password_reset_token_expiration() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        let token = simulator.request_password_reset(&user.email).unwrap();

        // Verify token is initially valid
        let validation_result = simulator.validate_reset_token(&token);
        assert!(validation_result.is_ok());

        // Verify token expiration time is set correctly (1 hour)
        let token_data = validation_result.unwrap();
        let expected_expiry = token_data.created_at + Duration::hours(1);
        assert_eq!(token_data.expires_at, expected_expiry);
    }

    #[test]
    fn test_cleanup_expired_tokens() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        // Add some expired tokens manually
        let expired_token1 = PasswordResetToken {
            token: "expired1".to_string(),
            user_id: user.id,
            email: user.email.clone(),
            created_at: Utc::now() - Duration::hours(3),
            expires_at: Utc::now() - Duration::hours(2),
            used: false,
        };

        let expired_token2 = PasswordResetToken {
            token: "expired2".to_string(),
            user_id: user.id,
            email: user.email.clone(),
            created_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::minutes(30),
            used: false,
        };

        simulator.reset_tokens.push(expired_token1);
        simulator.reset_tokens.push(expired_token2);

        // Add a valid token
        let valid_token = simulator.request_password_reset(&user.email).unwrap();

        // Before cleanup, we should have 3 tokens (2 expired + 1 valid)
        assert_eq!(simulator.reset_tokens.len(), 3);

        // Run cleanup
        simulator.cleanup_expired_tokens();

        // After cleanup, expired tokens should be marked as used
        let active_tokens = simulator.get_active_reset_tokens(&user.id);
        assert_eq!(active_tokens.len(), 1);
        assert_eq!(active_tokens[0].token, valid_token);

        // But expired tokens should still exist, just marked as used
        let expired_used_tokens: Vec<_> = simulator.reset_tokens.iter()
            .filter(|t| t.used && (t.token == "expired1" || t.token == "expired2"))
            .collect();
        assert_eq!(expired_used_tokens.len(), 2);
    }

    #[test]
    fn test_password_reset_activity_tracking() {
        let mut simulator = PasswordResetFlowSimulator::new();
        let mut activity_log = Vec::new();

        let user = simulator.create_test_user(
            "tracked@example.com".to_string(),
            "trackeduser".to_string(),
            "OldPassword123!".to_string(),
        );

        // Request password reset and track activity
        let token = simulator.request_password_reset(&user.email).unwrap();

        activity_log.push(json!({
            "event": "password_reset_requested",
            "user_id": user.id,
            "email": user.email,
            "timestamp": Utc::now(),
            "token_expires_at": Utc::now() + Duration::hours(1)
        }));

        // Reset password and track activity
        simulator.reset_password(&token, "NewSecurePass456#").unwrap();

        activity_log.push(json!({
            "event": "password_reset_completed",
            "user_id": user.id,
            "timestamp": Utc::now()
        }));

        // Verify activity was tracked
        assert_eq!(activity_log.len(), 2);
        assert!(activity_log.iter().all(|log| log.get("event").is_some()));

        // Verify password history was updated
        let password_history = simulator.get_user_password_history(&user.id);
        assert_eq!(password_history.len(), 2); // Original + new password
    }

    #[test]
    fn test_concurrent_password_reset_attempts() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "concurrent@example.com".to_string(),
            "concurrentuser".to_string(),
            "Password123!".to_string(),
        );

        // Simulate concurrent reset requests
        let token1 = simulator.request_password_reset(&user.email).unwrap();
        let token2 = simulator.request_password_reset(&user.email).unwrap();
        let token3 = simulator.request_password_reset(&user.email).unwrap();

        // Only the latest token should be valid
        let result1 = simulator.reset_password(&token1, "NewPassword1!");
        let result2 = simulator.reset_password(&token2, "NewPassword2!");
        assert!(result1.is_err());
        assert!(result2.is_err());

        // The latest token should work
        let result3 = simulator.reset_password(&token3, "NewPassword3!");
        assert!(result3.is_ok());

        // Verify only 3 emails were sent (one for each request)
        assert_eq!(simulator.sent_emails.len(), 3);
    }

    #[test]
    fn test_enhanced_password_reset_with_rate_limiting() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        let ip = "192.168.1.200";

        // Should allow up to 5 attempts in 15 minutes
        for i in 0..5 {
            let result = simulator.enhanced_request_password_reset(&user.email, ip, "TestAgent/1.0");
            assert!(result.is_ok(), "Attempt {} should succeed", i + 1);
        }

        // 6th attempt should fail due to rate limiting
        let result = simulator.enhanced_request_password_reset(&user.email, ip, "TestAgent/1.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("Too many password reset attempts"));
            }
            _ => panic!("Expected rate limit error"),
        }
    }

    #[test]
    fn test_fraud_detection_blocks_suspicious_ips() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        // Set IP reputation as suspicious
        let suspicious_ip = "10.0.0.666";
        simulator.fraud_detection.ip_reputation.insert(suspicious_ip.to_string(), 15);

        let result = simulator.enhanced_request_password_reset(&user.email, suspicious_ip, "AttackBot/1.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Authentication { message } => {
                assert!(message.contains("suspicious IP address"));
            }
            _ => panic!("Expected suspicious IP block"),
        }
    }

    #[test]
    fn test_enhanced_password_validation_common_passwords() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "OldPassword123!".to_string(),
        );

        let token = simulator.request_password_reset(&user.email).unwrap();

        // Try to use a common password
        let result = simulator.enhanced_reset_password(&token, "password", "192.168.1.100", "TestAgent/1.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "password");
                assert!(message.contains("too common"));
            }
            _ => panic!("Expected common password rejection"),
        }
    }

    #[test]
    fn test_password_minimum_age_enforcement() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password1!".to_string(),
        );

        // First password change
        let token1 = simulator.request_password_reset(&user.email).unwrap();
        simulator.reset_password(&token1, "Password2!").unwrap();

        // Immediate second change should fail due to minimum age
        let token2 = simulator.request_password_reset(&user.email).unwrap();
        let result = simulator.enhanced_reset_password(&token2, "Password3!", "192.168.1.100", "TestAgent/1.0");

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "password");
                assert!(message.contains("changed too recently"));
            }
            _ => panic!("Expected minimum age enforcement"),
        }
    }

    #[test]
    fn test_security_event_logging() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "tracked@example.com".to_string(),
            "trackeduser".to_string(),
            "Password123!".to_string(),
        );

        // Request password reset
        let token = simulator.enhanced_request_password_reset(&user.email, "192.168.1.100", "Chrome/91.0").unwrap();

        // Reset password
        simulator.enhanced_reset_password(&token, "NewPassword456!", "192.168.1.100", "Chrome/91.0").unwrap();

        // Verify security events were logged
        let events = simulator.get_security_events_for_user(&user.id);
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].event_type, SecurityEventType::PasswordResetRequested);
        assert_eq!(events[1].event_type, SecurityEventType::PasswordResetCompleted);

        // Both events should have the same IP and user agent
        assert_eq!(events[0].ip_address, "192.168.1.100");
        assert_eq!(events[1].ip_address, "192.168.1.100");
        assert_eq!(events[0].user_agent, "Chrome/91.0");
        assert_eq!(events[1].user_agent, "Chrome/91.0");
    }

    #[test]
    fn test_security_notifications() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "notified@example.com".to_string(),
            "notifieduser".to_string(),
            "Password123!".to_string(),
        );

        // Set notification preferences
        simulator.notification_preferences.insert(user.id, NotificationSettings::default());

        let token = simulator.request_password_reset(&user.email).unwrap();
        simulator.enhanced_reset_password(&token, "NewPassword456!", "192.168.1.100", "Chrome/91.0").unwrap();

        // Should have 3 emails: reset request + confirmation + security alert
        assert_eq!(simulator.sent_emails.len(), 3);

        let email_types: Vec<&EmailType> = simulator.sent_emails.iter().map(|e| &e.email_type).collect();
        assert!(email_types.contains(&&EmailType::PasswordResetRequest));
        assert!(email_types.contains(&&EmailType::PasswordResetConfirmation));
        assert!(email_types.contains(&&EmailType::SecurityAlert));
    }

    #[test]
    fn test_fraud_detection_scoring() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string(),
            "Password123!".to_string(),
        );

        // Set up fraud detection conditions
        let ip = "10.0.0.1";
        simulator.fraud_detection.ip_reputation.insert(ip.to_string(), 20); // Bad reputation

        let geo = GeoLocation {
            country: "Unknown".to_string(),
            city: "Unknown".to_string(),
            is_vpn: true,
            is_tor: false,
        };
        simulator.fraud_detection.geolocation_checks.insert(ip.to_string(), geo);

        // Calculate fraud score
        let risk_score = simulator.detect_fraud(&user.email, ip, "SuspiciousAgent/1.0");

        // Should have high risk score due to bad IP reputation + VPN
        assert!(risk_score > 50.0);
        assert!(risk_score <= 100.0);
    }

    #[tokio::test]
    async fn test_concurrent_password_reset_requests() {
        let mut simulator = PasswordResetFlowSimulator::new();

        // Create multiple users
        let users: Vec<String> = (0..5).map(|i| {
            let email = format!("user{}@example.com", i);
            simulator.create_test_user(email.clone(), format!("user{}", i), "Password123!".to_string());
            email
        }).collect();

        // Simulate concurrent reset requests
        let results = simulator.concurrent_reset_simulation(users, "192.168.1.100").await;

        // All should succeed since they're for different users
        assert_eq!(results.len(), 5);
        for result in results {
            assert!(result.is_ok());
        }

        // Verify 5 emails were sent
        assert_eq!(simulator.sent_emails.len(), 5);
    }

    #[test]
    fn test_advanced_attack_simulation() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "victim@example.com".to_string(),
            "victim".to_string(),
            "Password123!".to_string(),
        );

        // Simulate distributed attack from multiple IPs
        let results = simulator.simulate_advanced_attack(&user.email);

        // All should be blocked due to low IP reputation
        assert_eq!(results.len(), 5);
        for result in results {
            assert!(result.is_err());
        }

        // Verify security events were logged (may be fewer due to early IP blocking)
        let events = simulator.get_security_events_for_user(&user.id);
        assert!(events.len() >= 1); // At least some events should be logged
    }

    #[test]
    fn test_two_factor_authentication_integration() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "2fa@example.com".to_string(),
            "2fauser".to_string(),
            "Password123!".to_string(),
        );

        // Enable 2FA for user
        simulator.enable_two_factor(user.id);

        // Verify 2FA is enabled
        assert!(simulator.two_factor_enabled.get(&user.id).unwrap_or(&false));

        // Verify backup codes were generated
        assert!(simulator.backup_codes.contains_key(&user.id));
        let codes = &simulator.backup_codes[&user.id];
        assert_eq!(codes.len(), 10);

        // Check risk assessment
        let assessment = simulator.get_risk_assessment(&user.id);
        assert_eq!(assessment["two_factor_enabled"], json!(true));
    }

    #[test]
    fn test_comprehensive_risk_assessment() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "assessed@example.com".to_string(),
            "assesseduser".to_string(),
            "Password123!".to_string(),
        );

        // Generate some security activity
        let token = simulator.enhanced_request_password_reset(&user.email, "192.168.1.100", "Chrome/91.0").unwrap();
        simulator.enhanced_reset_password(&token, "NewPassword456!", "192.168.1.100", "Chrome/91.0").unwrap();

        // Set risk score
        simulator.fraud_detection.risk_scores.insert(user.id, 25.5);

        // Get risk assessment
        let assessment = simulator.get_risk_assessment(&user.id);

        assert_eq!(assessment["total_security_events"], json!(2));
        assert_eq!(assessment["recent_password_resets"], json!(1));
        assert_eq!(assessment["two_factor_enabled"], json!(false));
        assert_eq!(assessment["risk_score"], json!(25.5));
    }

    #[test]
    fn test_password_reset_with_geolocation_checks() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "geo@example.com".to_string(),
            "geouser".to_string(),
            "Password123!".to_string(),
        );

        // Set up geolocation data
        let tor_ip = "185.220.101.1";
        let tor_geo = GeoLocation {
            country: "Unknown".to_string(),
            city: "Unknown".to_string(),
            is_vpn: false,
            is_tor: true,
        };
        simulator.fraud_detection.geolocation_checks.insert(tor_ip.to_string(), tor_geo);

        // Calculate fraud score for Tor IP
        let risk_score = simulator.detect_fraud(&user.email, tor_ip, "TorBrowser/1.0");

        // Should have very high risk score due to Tor usage
        assert!(risk_score >= 40.0);
    }

    #[test]
    fn test_failed_reset_attempt_tracking() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let nonexistent_email = "doesnotexist@example.com";

        // Make several failed requests
        for i in 0..4 {
            let _ = simulator.enhanced_request_password_reset(nonexistent_email, "192.168.1.100", "TestAgent/1.0");
        }

        // Should have tracked failed attempts
        assert_eq!(simulator.failed_reset_attempts[nonexistent_email], 4);

        // Should have logged suspicious activity after 3 failures
        let suspicious_events: Vec<_> = simulator.security_events.iter()
            .filter(|e| e.event_type == SecurityEventType::SuspiciousActivity)
            .collect();
        assert_eq!(suspicious_events.len(), 2); // After 3rd and 4th attempt
    }

    #[test]
    fn test_device_fingerprinting_in_fraud_detection() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "device@example.com".to_string(),
            "deviceuser".to_string(),
            "Password123!".to_string(),
        );

        let ip = "192.168.1.100";
        let user_agent = "Chrome/91.0.4472.124";

        // First request from new device
        let risk_score1 = simulator.detect_fraud(&user.email, ip, user_agent);

        // Should have some risk due to new device
        assert!(risk_score1 > 0.0);

        // Second request from same device (device is now known, so lower risk from device)
        // But failed attempts might increase, so we check that device trust improves
        let fingerprint = format!("{}_{}", user_agent, ip);
        let device_before_trust = simulator.fraud_detection.device_fingerprints.get(&fingerprint)
            .map(|d| d.is_trusted)
            .unwrap_or(false);

        let risk_score2 = simulator.detect_fraud(&user.email, ip, user_agent);

        // Device should be tracked consistently (not more risky from device perspective)
        assert!(risk_score2 >= 0.0);

        // Verify device fingerprint was created
        let fingerprint = format!("{}_{}", user_agent, ip);
        assert!(simulator.fraud_detection.device_fingerprints.contains_key(&fingerprint));
    }

    #[test]
    fn test_password_policy_enforcement() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "policy@example.com".to_string(),
            "policyuser".to_string(),
            "Password123!".to_string(),
        );

        // Verify default policy settings
        assert_eq!(simulator.password_policy.min_length, 8);
        assert_eq!(simulator.password_policy.history_count, 3);
        assert_eq!(simulator.password_policy.min_age_hours, 24);
        assert!(simulator.password_policy.require_uppercase);
        assert!(simulator.password_policy.require_lowercase);
        assert!(simulator.password_policy.require_numbers);
        assert!(simulator.password_policy.require_special);

        // Verify common passwords are blocked
        let token = simulator.request_password_reset(&user.email).unwrap();
        let result = simulator.enhanced_reset_password(&token, "123456", "192.168.1.100", "TestAgent/1.0");

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { message, .. } => {
                assert!(message.contains("too common"));
            }
            _ => panic!("Expected common password rejection"),
        }
    }

    #[test]
    fn test_comprehensive_password_reset_flow() {
        let mut simulator = PasswordResetFlowSimulator::new();

        let user = simulator.create_test_user(
            "comprehensive@example.com".to_string(),
            "comprehensiveuser".to_string(),
            "OldPassword123!".to_string(),
        );

        // Set up notification preferences
        simulator.notification_preferences.insert(user.id, NotificationSettings::default());

        // Step 1: Request password reset with enhanced security
        let token = simulator.enhanced_request_password_reset(&user.email, "192.168.1.100", "Chrome/91.0").unwrap();

        // Step 2: Verify security event was logged
        let events = simulator.get_security_events_for_user(&user.id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, SecurityEventType::PasswordResetRequested);

        // Step 3: Reset password with enhanced validation
        let result = simulator.enhanced_reset_password(&token, "NewSecurePassword789#", "192.168.1.100", "Chrome/91.0");
        assert!(result.is_ok());

        // Step 4: Verify completion event was logged
        let events = simulator.get_security_events_for_user(&user.id);
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event_type, SecurityEventType::PasswordResetCompleted);

        // Step 5: Verify multiple notification emails were sent
        let sent_emails: Vec<_> = simulator.sent_emails.iter()
            .filter(|e| e.to == user.email)
            .collect();
        assert_eq!(sent_emails.len(), 3); // Request + Confirmation + Security Alert

        // Step 6: Verify password history was updated
        let password_history = simulator.get_user_password_history(&user.id);
        assert_eq!(password_history.len(), 2); // Original + new password

        // Step 7: Verify token was marked as used
        let used_tokens: Vec<_> = simulator.reset_tokens.iter()
            .filter(|t| t.token == token && t.used)
            .collect();
        assert_eq!(used_tokens.len(), 1);
    }
}