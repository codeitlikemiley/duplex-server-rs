use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Model;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "PascalCase")]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl Model for User {}

impl User {
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            username,
            email,
            password_hash,
            email_verified: false,
            status: UserStatus::PendingVerification,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        }
    }

    pub fn verify_email(&mut self) {
        self.email_verified = true;
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();
    }

    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();
    }

    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
        self.updated_at = Utc::now();
    }

    pub fn deactivate(&mut self) {
        self.status = UserStatus::Inactive;
        self.updated_at = Utc::now();
    }

    pub fn update_login_timestamp(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn change_password(&mut self, new_password_hash: String) {
        self.password_hash = new_password_hash;
        self.updated_at = Utc::now();
    }

    pub fn change_email(&mut self, new_email: String) {
        self.email = new_email;
        self.email_verified = false; // Require re-verification
        self.updated_at = Utc::now();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    pub fn is_verified(&self) -> bool {
        self.email_verified
    }

    pub fn can_login(&self) -> bool {
        self.is_active() && self.is_verified()
    }

    pub fn account_age_days(&self) -> i64 {
        let now = Utc::now();
        now.signed_duration_since(self.created_at).num_days()
    }

    pub fn days_since_last_login(&self) -> Option<i64> {
        self.last_login_at.map(|last_login| {
            let now = Utc::now();
            now.signed_duration_since(last_login).num_days()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_user_creation() {
        let username = "testuser".to_string();
        let email = "test@example.com".to_string();
        let password_hash = "hashed_password".to_string();

        let user = User::new(username.clone(), email.clone(), password_hash.clone());

        assert_eq!(user.username, username);
        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert!(!user.email_verified);
        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(user.last_login_at.is_none());

        // UUID should be valid v7
        assert!(user.id.get_version_num() == 7);

        // Timestamps should be recent
        let now = Utc::now();
        assert!((now - user.created_at).num_seconds() < 1);
        assert!((now - user.updated_at).num_seconds() < 1);
    }

    #[test]
    fn test_email_verification() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        let initial_updated_at = user.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        user.verify_email();

        assert!(user.email_verified);
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_user_status_transitions() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        // Test activation
        user.activate();
        assert_eq!(user.status, UserStatus::Active);

        // Test suspension
        user.suspend();
        assert_eq!(user.status, UserStatus::Suspended);

        // Test deactivation
        user.deactivate();
        assert_eq!(user.status, UserStatus::Inactive);

        // Test reactivation
        user.activate();
        assert_eq!(user.status, UserStatus::Active);
    }

    #[test]
    fn test_login_timestamp_update() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        assert!(user.last_login_at.is_none());

        let before_login = Utc::now();
        user.update_login_timestamp();
        let after_login = Utc::now();

        assert!(user.last_login_at.is_some());
        let login_time = user.last_login_at.unwrap();
        assert!(login_time >= before_login && login_time <= after_login);
    }

    #[test]
    fn test_password_change() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "old_password_hash".to_string(),
        );

        let initial_updated_at = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));

        let new_password_hash = "new_password_hash".to_string();
        user.change_password(new_password_hash.clone());

        assert_eq!(user.password_hash, new_password_hash);
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_email_change() {
        let mut user = User::new(
            "testuser".to_string(),
            "old@example.com".to_string(),
            "password_hash".to_string(),
        );

        // Verify email first
        user.verify_email();
        assert!(user.email_verified);

        let initial_updated_at = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));

        let new_email = "new@example.com".to_string();
        user.change_email(new_email.clone());

        assert_eq!(user.email, new_email);
        assert!(!user.email_verified); // Should require re-verification
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_user_state_queries() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password_hash".to_string(),
        );

        // Initially pending verification
        assert!(!user.is_active());
        assert!(!user.is_verified());
        assert!(!user.can_login());

        // After email verification
        user.verify_email();
        assert!(user.is_active());
        assert!(user.is_verified());
        assert!(user.can_login());

        // After suspension
        user.suspend();
        assert!(!user.is_active());
        assert!(user.is_verified()); // Email still verified
        assert!(!user.can_login());

        // After reactivation
        user.activate();
        assert!(user.is_active());
        assert!(user.is_verified());
        assert!(user.can_login());

        // After deactivation
        user.deactivate();
        assert!(!user.is_active());
        assert!(user.is_verified());
        assert!(!user.can_login());
    }

    #[test]
    fn test_account_age_calculation() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password_hash".to_string(),
        );

        // Account just created
        assert_eq!(user.account_age_days(), 0);

        // Simulate account created 5 days ago
        user.created_at = Utc::now() - Duration::days(5);
        assert_eq!(user.account_age_days(), 5);

        // Simulate account created 30 days ago
        user.created_at = Utc::now() - Duration::days(30);
        assert_eq!(user.account_age_days(), 30);
    }

    #[test]
    fn test_days_since_last_login() {
        let mut user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password_hash".to_string(),
        );

        // No login yet
        assert!(user.days_since_last_login().is_none());

        // Update login timestamp
        user.update_login_timestamp();
        assert_eq!(user.days_since_last_login(), Some(0));

        // Simulate login 3 days ago
        user.last_login_at = Some(Utc::now() - Duration::days(3));
        assert_eq!(user.days_since_last_login(), Some(3));

        // Simulate login 10 days ago
        user.last_login_at = Some(Utc::now() - Duration::days(10));
        assert_eq!(user.days_since_last_login(), Some(10));
    }

    #[test]
    fn test_user_status_serialization() {
        // Test that UserStatus can be serialized/deserialized
        let statuses = vec![
            UserStatus::Active,
            UserStatus::Inactive,
            UserStatus::Suspended,
            UserStatus::PendingVerification,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: UserStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password_hash".to_string(),
        );

        let serialized = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&serialized).unwrap();

        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.password_hash, deserialized.password_hash);
        assert_eq!(user.email_verified, deserialized.email_verified);
        assert_eq!(user.status, deserialized.status);
    }

    #[test]
    fn test_user_clone() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password_hash".to_string(),
        );

        let cloned_user = user.clone();

        assert_eq!(user.id, cloned_user.id);
        assert_eq!(user.username, cloned_user.username);
        assert_eq!(user.email, cloned_user.email);
        assert_eq!(user.password_hash, cloned_user.password_hash);
        assert_eq!(user.email_verified, cloned_user.email_verified);
        assert_eq!(user.status, cloned_user.status);
        assert_eq!(user.created_at, cloned_user.created_at);
        assert_eq!(user.updated_at, cloned_user.updated_at);
        assert_eq!(user.last_login_at, cloned_user.last_login_at);
    }

    #[test]
    fn test_edge_cases() {
        // Test with empty strings
        let user = User::new(
            String::new(),
            String::new(),
            String::new(),
        );

        assert_eq!(user.username, "");
        assert_eq!(user.email, "");
        assert_eq!(user.password_hash, "");

        // Test with very long strings
        let long_string = "a".repeat(1000);
        let user = User::new(
            long_string.clone(),
            format!("{}@example.com", long_string),
            long_string.clone(),
        );

        assert_eq!(user.username.len(), 1000);
        assert_eq!(user.password_hash.len(), 1000);
    }

    #[test]
    fn test_user_lifecycle() {
        let mut user = User::new(
            "lifecycle_user".to_string(),
            "lifecycle@example.com".to_string(),
            "password_hash".to_string(),
        );

        // Start as pending verification
        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(!user.email_verified);
        assert!(!user.can_login());

        // Verify email and activate
        user.verify_email();
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.email_verified);
        assert!(user.can_login());

        // Simulate login
        user.update_login_timestamp();
        assert!(user.last_login_at.is_some());

        // Change password
        let new_password = "new_password_hash".to_string();
        user.change_password(new_password.clone());
        assert_eq!(user.password_hash, new_password);

        // Change email (requires re-verification)
        let new_email = "new_email@example.com".to_string();
        user.change_email(new_email.clone());
        assert_eq!(user.email, new_email);
        assert!(!user.email_verified);
        assert!(!user.can_login()); // Can't login without verification

        // Re-verify email
        user.verify_email();
        assert!(user.can_login());

        // Suspend account
        user.suspend();
        assert_eq!(user.status, UserStatus::Suspended);
        assert!(!user.can_login());

        // Reactivate
        user.activate();
        assert!(user.can_login());

        // Deactivate
        user.deactivate();
        assert_eq!(user.status, UserStatus::Inactive);
        assert!(!user.can_login());
    }
}