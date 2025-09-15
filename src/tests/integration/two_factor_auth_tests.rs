//! Integration tests for two-factor authentication operations

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
    RateLimitExceeded {
        retry_after: Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSetup {
    pub user_id: Uuid,
    pub secret: String,
    pub qr_code_url: String,
    pub backup_codes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCode {
    pub code: String,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct TwoFactorAttempt {
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub method: TwoFactorMethod,
    pub ip_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwoFactorMethod {
    TOTP,
    BackupCode,
    SMS,
    Email,
}

#[derive(Debug, Clone)]
pub struct TrustedDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_fingerprint: String,
    pub device_name: String,
    pub trusted_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct TwoFactorAuthSimulator {
    users: Vec<User>,
    two_factor_setups: HashMap<Uuid, TwoFactorSetup>,
    backup_codes: HashMap<Uuid, Vec<BackupCode>>,
    two_factor_attempts: Vec<TwoFactorAttempt>,
    trusted_devices: HashMap<Uuid, Vec<TrustedDevice>>,
    rate_limiter: HashMap<Uuid, Vec<DateTime<Utc>>>,
    max_attempts_per_hour: usize,
    backup_code_count: usize,
    totp_window: i64,
    trusted_device_duration: Duration,
}

impl TwoFactorAuthSimulator {
    pub fn new() -> Self {
        Self {
            users: vec![],
            two_factor_setups: HashMap::new(),
            backup_codes: HashMap::new(),
            two_factor_attempts: vec![],
            trusted_devices: HashMap::new(),
            rate_limiter: HashMap::new(),
            max_attempts_per_hour: 5,
            backup_code_count: 10,
            totp_window: 30,
            trusted_device_duration: Duration::days(30),
        }
    }

    pub async fn create_test_user(&mut self, two_factor_enabled: bool) -> Uuid {
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
            two_factor_enabled,
            two_factor_secret: if two_factor_enabled {
                Some(self.generate_secret())
            } else {
                None
            },
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

    fn generate_secret(&self) -> String {
        format!("SECRET_{}", Uuid::now_v7())
    }

    fn generate_backup_code(&self) -> String {
        Self::generate_backup_code_static()
    }

    fn generate_backup_code_static() -> String {
        format!("{:08}", rand::random::<u32>() % 100000000)
    }

    fn generate_totp_code(secret: &str, timestamp: DateTime<Utc>, totp_window: i64) -> String {
        // Simplified TOTP generation for testing
        let time_step = timestamp.timestamp() / totp_window;
        let hash = format!("{}{}", secret, time_step);
        format!("{:06}", hash.len() % 1000000)
    }

    pub async fn setup_two_factor(&mut self, user_id: Uuid) -> Result<TwoFactorSetup, AppError> {
        // Check if user exists and if 2FA is already enabled, get email
        let user_email = {
            let user = self.users.iter()
                .find(|u| u.id == user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            if user.two_factor_enabled {
                return Err(AppError::BadRequest {
                    message: "Two-factor authentication is already enabled".to_string(),
                });
            }

            user.email.clone()
        };

        // Generate secret and backup codes
        let secret = self.generate_secret();
        let backup_code_count = self.backup_code_count;
        let backup_codes: Vec<BackupCode> = (0..backup_code_count)
            .map(|_| BackupCode {
                code: Self::generate_backup_code_static(),
                used: false,
                used_at: None,
            })
            .collect();

        let setup = TwoFactorSetup {
            user_id,
            secret: secret.clone(),
            qr_code_url: format!("otpauth://totp/App:{}?secret={}", user_email, secret),
            backup_codes: backup_codes.iter().map(|c| c.code.clone()).collect(),
            created_at: Utc::now(),
            verified: false,
        };

        self.two_factor_setups.insert(user_id, setup.clone());
        self.backup_codes.insert(user_id, backup_codes);

        Ok(setup)
    }

    pub async fn verify_two_factor_setup(
        &mut self,
        user_id: Uuid,
        code: &str,
    ) -> Result<Vec<String>, AppError> {
        // Check rate limiting
        self.check_rate_limit(user_id)?;

        // Get setup details
        let (secret, backup_codes) = {
            let setup = self.two_factor_setups.get(&user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "2fa_setup".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            if setup.verified {
                return Err(AppError::BadRequest {
                    message: "Two-factor authentication is already verified".to_string(),
                });
            }

            (setup.secret.clone(), setup.backup_codes.clone())
        };

        // Verify TOTP code
        let expected_code = Self::generate_totp_code(&secret, Utc::now(), self.totp_window);

        if code != expected_code {
            self.record_attempt(user_id, false, TwoFactorMethod::TOTP, "127.0.0.1");
            return Err(AppError::Unauthorized {
                message: "Invalid verification code".to_string(),
            });
        }

        // Mark as verified and enable 2FA
        if let Some(setup) = self.two_factor_setups.get_mut(&user_id) {
            setup.verified = true;
        }

        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.two_factor_enabled = true;
            user.two_factor_secret = Some(secret);
        }

        self.record_attempt(user_id, true, TwoFactorMethod::TOTP, "127.0.0.1");

        // Return backup codes
        Ok(backup_codes)
    }

    pub async fn verify_two_factor_login(
        &mut self,
        user_id: Uuid,
        code: &str,
        ip_address: &str,
    ) -> Result<(), AppError> {
        // Check rate limiting
        self.check_rate_limit(user_id)?;

        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if !user.two_factor_enabled {
            return Err(AppError::BadRequest {
                message: "Two-factor authentication is not enabled".to_string(),
            });
        }

        let secret = user.two_factor_secret.as_ref()
            .ok_or_else(|| AppError::InternalServerError {
                message: "Two-factor secret not found".to_string(),
            })?;

        // Try TOTP code first
        let expected_code = Self::generate_totp_code(secret, Utc::now(), self.totp_window);

        if code == expected_code {
            self.record_attempt(user_id, true, TwoFactorMethod::TOTP, ip_address);
            return Ok(());
        }

        // Try backup code
        if let Some(backup_codes) = self.backup_codes.get_mut(&user_id) {
            if let Some(backup_code) = backup_codes.iter_mut().find(|c| c.code == code && !c.used) {
                backup_code.used = true;
                backup_code.used_at = Some(Utc::now());
                self.record_attempt(user_id, true, TwoFactorMethod::BackupCode, ip_address);
                return Ok(());
            }
        }

        self.record_attempt(user_id, false, TwoFactorMethod::TOTP, ip_address);
        Err(AppError::Unauthorized {
            message: "Invalid authentication code".to_string(),
        })
    }

    pub async fn disable_two_factor(
        &mut self,
        user_id: Uuid,
        password: &str,
    ) -> Result<(), AppError> {
        let user = self.users.iter_mut()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        // Verify password (simplified)
        if password != "correct_password" {
            return Err(AppError::Unauthorized {
                message: "Invalid password".to_string(),
            });
        }

        if !user.two_factor_enabled {
            return Err(AppError::BadRequest {
                message: "Two-factor authentication is not enabled".to_string(),
            });
        }

        // Disable 2FA
        user.two_factor_enabled = false;
        user.two_factor_secret = None;

        // Remove setup and backup codes
        self.two_factor_setups.remove(&user_id);
        self.backup_codes.remove(&user_id);

        // Remove trusted devices
        self.trusted_devices.remove(&user_id);

        Ok(())
    }

    pub async fn regenerate_backup_codes(
        &mut self,
        user_id: Uuid,
        current_code: &str,
    ) -> Result<Vec<String>, AppError> {
        // Verify current 2FA code
        self.verify_two_factor_login(user_id, current_code, "127.0.0.1").await?;

        // Generate new backup codes
        let new_codes: Vec<BackupCode> = (0..self.backup_code_count)
            .map(|_| BackupCode {
                code: self.generate_backup_code(),
                used: false,
                used_at: None,
            })
            .collect();

        let code_strings: Vec<String> = new_codes.iter().map(|c| c.code.clone()).collect();

        self.backup_codes.insert(user_id, new_codes);

        Ok(code_strings)
    }

    pub async fn get_backup_codes_status(&self, user_id: Uuid) -> Result<(usize, usize), AppError> {
        let codes = self.backup_codes.get(&user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "backup_codes".to_string(),
                id: Some(user_id.to_string()),
            })?;

        let total = codes.len();
        let remaining = codes.iter().filter(|c| !c.used).count();

        Ok((remaining, total))
    }

    pub async fn add_trusted_device(
        &mut self,
        user_id: Uuid,
        device_fingerprint: String,
        device_name: String,
    ) -> Result<Uuid, AppError> {
        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if !user.two_factor_enabled {
            return Err(AppError::BadRequest {
                message: "Two-factor authentication must be enabled to add trusted devices".to_string(),
            });
        }

        let device = TrustedDevice {
            id: Uuid::now_v7(),
            user_id,
            device_fingerprint,
            device_name,
            trusted_at: Utc::now(),
            last_used: Utc::now(),
            expires_at: Utc::now() + self.trusted_device_duration,
        };

        let device_id = device.id;

        self.trusted_devices.entry(user_id)
            .or_insert_with(Vec::new)
            .push(device);

        Ok(device_id)
    }

    pub async fn is_trusted_device(
        &mut self,
        user_id: Uuid,
        device_fingerprint: &str,
    ) -> bool {
        if let Some(devices) = self.trusted_devices.get_mut(&user_id) {
            if let Some(device) = devices.iter_mut()
                .find(|d| d.device_fingerprint == device_fingerprint)
            {
                if Utc::now() <= device.expires_at {
                    device.last_used = Utc::now();
                    return true;
                }
            }
        }
        false
    }

    pub async fn remove_trusted_device(
        &mut self,
        user_id: Uuid,
        device_id: Uuid,
    ) -> Result<(), AppError> {
        let devices = self.trusted_devices.get_mut(&user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "trusted_devices".to_string(),
                id: Some(user_id.to_string()),
            })?;

        let initial_len = devices.len();
        devices.retain(|d| d.id != device_id);

        if devices.len() == initial_len {
            return Err(AppError::NotFound {
                resource: "device".to_string(),
                id: Some(device_id.to_string()),
            });
        }

        Ok(())
    }

    pub async fn get_trusted_devices(&self, user_id: Uuid) -> Vec<TrustedDevice> {
        self.trusted_devices.get(&user_id)
            .map(|devices| devices.clone())
            .unwrap_or_default()
    }

    pub async fn require_two_factor_for_sensitive_operation(
        &mut self,
        user_id: Uuid,
        code: &str,
        operation: &str,
    ) -> Result<(), AppError> {
        let user = self.users.iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            })?;

        if !user.two_factor_enabled {
            // Allow operation without 2FA if not enabled
            return Ok(());
        }

        // Verify 2FA code for sensitive operation
        self.verify_two_factor_login(user_id, code, "127.0.0.1").await?;

        // Log the sensitive operation
        println!("Sensitive operation '{}' authorized for user {}", operation, user_id);

        Ok(())
    }

    fn check_rate_limit(&mut self, user_id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);

        let attempts = self.rate_limiter.entry(user_id)
            .or_insert_with(Vec::new);

        // Clean old attempts
        attempts.retain(|&t| t > hour_ago);

        if attempts.len() >= self.max_attempts_per_hour {
            return Err(AppError::RateLimitExceeded {
                retry_after: Duration::hours(1),
            });
        }

        attempts.push(now);
        Ok(())
    }

    fn record_attempt(
        &mut self,
        user_id: Uuid,
        success: bool,
        method: TwoFactorMethod,
        ip_address: &str,
    ) {
        self.two_factor_attempts.push(TwoFactorAttempt {
            user_id,
            timestamp: Utc::now(),
            success,
            method,
            ip_address: ip_address.to_string(),
        });
    }

    pub async fn get_recent_attempts(&self, user_id: Uuid, limit: usize) -> Vec<TwoFactorAttempt> {
        self.two_factor_attempts.iter()
            .filter(|a| a.user_id == user_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_two_factor() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        let result = simulator.setup_two_factor(user_id).await;
        assert!(result.is_ok());

        let setup = result.unwrap();
        assert_eq!(setup.user_id, user_id);
        assert!(!setup.verified);
        assert_eq!(setup.backup_codes.len(), 10);
        assert!(setup.qr_code_url.contains("otpauth://totp"));
    }

    #[tokio::test]
    async fn test_verify_two_factor_setup() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        let secret = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            setup.secret.clone()
        };

        let code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);

        let result = simulator.verify_two_factor_setup(user_id, &code).await;
        assert!(result.is_ok());

        let backup_codes = result.unwrap();
        assert_eq!(backup_codes.len(), 10);

        // Verify user has 2FA enabled
        let user = simulator.users.iter().find(|u| u.id == user_id).unwrap();
        assert!(user.two_factor_enabled);
    }

    #[tokio::test]
    async fn test_two_factor_login_with_totp() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        let secret = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let secret = setup.secret.clone();
            let code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
            secret
        };

        // Test login with TOTP
        let login_code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
        let result = simulator.verify_two_factor_login(user_id, &login_code, "192.168.1.1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_two_factor_login_with_backup_code() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        let backup_codes = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap()
        };

        // Test login with backup code
        let result = simulator.verify_two_factor_login(user_id, &backup_codes[0], "192.168.1.1").await;
        assert!(result.is_ok());

        // Verify backup code is marked as used
        let codes = simulator.backup_codes.get(&user_id).unwrap();
        assert!(codes[0].used);
    }

    #[tokio::test]
    async fn test_backup_code_reuse_prevention() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        let backup_codes = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap()
        };

        // Use backup code once
        simulator.verify_two_factor_login(user_id, &backup_codes[0], "192.168.1.1").await.unwrap();

        // Try to reuse the same backup code
        let result = simulator.verify_two_factor_login(user_id, &backup_codes[0], "192.168.1.1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Unauthorized { .. }));
    }

    #[tokio::test]
    async fn test_disable_two_factor() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
        }

        // Disable 2FA
        let result = simulator.disable_two_factor(user_id, "correct_password").await;
        assert!(result.is_ok());

        // Verify user has 2FA disabled
        let user = simulator.users.iter().find(|u| u.id == user_id).unwrap();
        assert!(!user.two_factor_enabled);
        assert!(user.two_factor_secret.is_none());
    }

    #[tokio::test]
    async fn test_regenerate_backup_codes() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        let (secret, original_codes) = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let secret = setup.secret.clone();
            let code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
            let codes = simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
            (secret, codes)
        };

        // Regenerate backup codes
        let totp_code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
        let new_codes = simulator.regenerate_backup_codes(user_id, &totp_code).await.unwrap();

        assert_eq!(new_codes.len(), 10);
        // Verify codes are different
        assert_ne!(original_codes[0], new_codes[0]);
    }

    #[tokio::test]
    async fn test_trusted_device_management() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
        }

        // Add trusted device
        let device_id = simulator.add_trusted_device(
            user_id,
            "fingerprint123".to_string(),
            "iPhone 12".to_string()
        ).await.unwrap();

        // Check if device is trusted
        assert!(simulator.is_trusted_device(user_id, "fingerprint123").await);

        // Get all trusted devices
        let devices = simulator.get_trusted_devices(user_id).await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_name, "iPhone 12");

        // Remove trusted device
        simulator.remove_trusted_device(user_id, device_id).await.unwrap();
        assert!(!simulator.is_trusted_device(user_id, "fingerprint123").await);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut simulator = TwoFactorAuthSimulator::new();
        simulator.max_attempts_per_hour = 3; // Lower limit for testing
        let user_id = simulator.create_test_user(true).await;

        // Make max attempts with wrong codes
        for _ in 0..3 {
            let _ = simulator.verify_two_factor_login(user_id, "wrong_code", "192.168.1.1").await;
        }

        // Next attempt should be rate limited
        let result = simulator.verify_two_factor_login(user_id, "any_code", "192.168.1.1").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::RateLimitExceeded { .. }));
    }

    #[tokio::test]
    async fn test_sensitive_operation_protection() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Without 2FA, sensitive operations are allowed
        let result = simulator.require_two_factor_for_sensitive_operation(
            user_id,
            "",
            "delete_account"
        ).await;
        assert!(result.is_ok());

        // Setup and verify 2FA
        let secret = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let secret = setup.secret.clone();
            let code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
            secret
        };

        // With 2FA enabled, wrong code fails
        let result = simulator.require_two_factor_for_sensitive_operation(
            user_id,
            "wrong_code",
            "delete_account"
        ).await;
        assert!(result.is_err());

        // With correct code, operation is allowed
        let valid_code = TwoFactorAuthSimulator::generate_totp_code(&secret, Utc::now(), 30);
        let result = simulator.require_two_factor_for_sensitive_operation(
            user_id,
            &valid_code,
            "delete_account"
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_backup_codes_status() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        let backup_codes = {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap()
        };

        // Check initial status
        let (remaining, total) = simulator.get_backup_codes_status(user_id).await.unwrap();
        assert_eq!(remaining, 10);
        assert_eq!(total, 10);

        // Use a backup code
        simulator.verify_two_factor_login(user_id, &backup_codes[0], "192.168.1.1").await.unwrap();

        // Check status after use
        let (remaining, total) = simulator.get_backup_codes_status(user_id).await.unwrap();
        assert_eq!(remaining, 9);
        assert_eq!(total, 10);
    }

    #[tokio::test]
    async fn test_recent_attempts_tracking() {
        let mut simulator = TwoFactorAuthSimulator::new();
        let user_id = simulator.create_test_user(true).await;

        // Make some attempts
        let _ = simulator.verify_two_factor_login(user_id, "wrong1", "192.168.1.1").await;
        let _ = simulator.verify_two_factor_login(user_id, "wrong2", "192.168.1.2").await;

        // Get recent attempts
        let attempts = simulator.get_recent_attempts(user_id, 10).await;
        assert_eq!(attempts.len(), 2);
        assert!(!attempts[0].success);
        assert_eq!(attempts[0].ip_address, "192.168.1.2");
        assert_eq!(attempts[1].ip_address, "192.168.1.1");
    }

    #[tokio::test]
    async fn test_trusted_device_expiration() {
        let mut simulator = TwoFactorAuthSimulator::new();
        simulator.trusted_device_duration = Duration::seconds(1); // Short duration for testing
        let user_id = simulator.create_test_user(false).await;

        // Setup and verify 2FA
        {
            let setup = simulator.setup_two_factor(user_id).await.unwrap();
            let code = TwoFactorAuthSimulator::generate_totp_code(&setup.secret, Utc::now(), 30);
            simulator.verify_two_factor_setup(user_id, &code).await.unwrap();
        }

        // Add trusted device
        simulator.add_trusted_device(
            user_id,
            "fingerprint123".to_string(),
            "Test Device".to_string()
        ).await.unwrap();

        // Device should be trusted initially
        assert!(simulator.is_trusted_device(user_id, "fingerprint123").await);

        // Manually expire the device
        if let Some(devices) = simulator.trusted_devices.get_mut(&user_id) {
            devices[0].expires_at = Utc::now() - Duration::hours(1);
        }

        // Device should no longer be trusted
        assert!(!simulator.is_trusted_device(user_id, "fingerprint123").await);
    }
}