use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::events::UserCreated;
use crate::domain::repositories::{UserProfileRepository, UserRepository};
use crate::models::{User, UserProfile, UserStatus};

pub mod fixtures {
    use super::*;

    pub fn create_test_user(id: Uuid, username: &str, email: &str) -> User {
        let now = Utc::now();
        User {
            id,
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "hashed_password".to_string(),
            email_verified: false,
            status: UserStatus::PendingVerification,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        }
    }

    pub fn create_active_user(id: Uuid, username: &str, email: &str) -> User {
        let now = Utc::now();
        User {
            id,
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "hashed_password".to_string(),
            email_verified: true,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
            last_login_at: Some(now),
        }
    }

    pub fn create_test_profile(user_id: Uuid, first_name: Option<&str>, last_name: Option<&str>) -> UserProfile {
        let now = Utc::now();
        UserProfile {
            user_id,
            first_name: first_name.map(|s| s.to_string()),
            last_name: last_name.map(|s| s.to_string()),
            bio: None,
            avatar_url: None,
            website: None,
            location: None,
            preferences: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn create_complete_profile(user_id: Uuid) -> UserProfile {
        let now = Utc::now();
        UserProfile {
            user_id,
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            bio: Some("Software developer".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            website: Some("https://johndoe.dev".to_string()),
            location: Some("San Francisco, CA".to_string()),
            preferences: serde_json::json!({"theme": "dark", "notifications": true}),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn create_test_event(user_id: Uuid, username: &str, email: &str) -> UserCreated {
        UserCreated {
            id: Uuid::now_v7(),
            user_id,
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MockUserRepository {
    users: Arc<Mutex<HashMap<Uuid, User>>>,
    users_by_email: Arc<Mutex<HashMap<String, User>>>,
    events: Arc<Mutex<Vec<UserCreated>>>,
    should_fail: Arc<Mutex<bool>>,
    save_count: Arc<Mutex<usize>>,
    find_count: Arc<Mutex<usize>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            users_by_email: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(Mutex::new(false)),
            save_count: Arc::new(Mutex::new(0)),
            find_count: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().await = should_fail;
    }

    pub async fn get_save_count(&self) -> usize {
        *self.save_count.lock().await
    }

    pub async fn get_find_count(&self) -> usize {
        *self.find_count.lock().await
    }

    pub async fn get_events(&self) -> Vec<UserCreated> {
        self.events.lock().await.clone()
    }

    pub async fn get_all_users(&self) -> Vec<User> {
        self.users.lock().await.values().cloned().collect()
    }

    pub async fn clear(&self) {
        self.users.lock().await.clear();
        self.users_by_email.lock().await.clear();
        self.events.lock().await.clear();
        *self.save_count.lock().await = 0;
        *self.find_count.lock().await = 0;
        *self.should_fail.lock().await = false;
    }

    pub async fn user_count(&self) -> usize {
        self.users.lock().await.len()
    }

    pub async fn has_user_with_email(&self, email: &str) -> bool {
        self.users_by_email.lock().await.contains_key(email)
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn save_user(&self, user: User) -> Result<(), sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.save_count.lock().await += 1;

        self.users.lock().await.insert(user.id, user.clone());
        self.users_by_email.lock().await.insert(user.email.clone(), user);
        Ok(())
    }

    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        self.events.lock().await.push(event);
        Ok(())
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.find_count.lock().await += 1;
        Ok(self.users.lock().await.get(&id).cloned())
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.find_count.lock().await += 1;
        Ok(self.users_by_email.lock().await.get(email).cloned())
    }
}

#[derive(Debug, Clone)]
pub struct MockUserProfileRepository {
    profiles: Arc<Mutex<HashMap<Uuid, UserProfile>>>,
    should_fail: Arc<Mutex<bool>>,
    save_count: Arc<Mutex<usize>>,
    find_count: Arc<Mutex<usize>>,
}

impl MockUserProfileRepository {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(Mutex::new(HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
            save_count: Arc::new(Mutex::new(0)),
            find_count: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().await = should_fail;
    }

    pub async fn get_save_count(&self) -> usize {
        *self.save_count.lock().await
    }

    pub async fn get_find_count(&self) -> usize {
        *self.find_count.lock().await
    }

    pub async fn get_all_profiles(&self) -> Vec<UserProfile> {
        self.profiles.lock().await.values().cloned().collect()
    }

    pub async fn clear(&self) {
        self.profiles.lock().await.clear();
        *self.save_count.lock().await = 0;
        *self.find_count.lock().await = 0;
        *self.should_fail.lock().await = false;
    }

    pub async fn profile_count(&self) -> usize {
        self.profiles.lock().await.len()
    }

    pub async fn has_profile_for_user(&self, user_id: Uuid) -> bool {
        self.profiles.lock().await.contains_key(&user_id)
    }
}

#[async_trait]
impl UserProfileRepository for MockUserProfileRepository {
    async fn save_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.save_count.lock().await += 1;
        self.profiles.lock().await.insert(profile.user_id, profile);
        Ok(())
    }

    async fn find_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<UserProfile>, sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.find_count.lock().await += 1;
        Ok(self.profiles.lock().await.get(&user_id).cloned())
    }

    async fn update_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.save_count.lock().await += 1;

        if self.profiles.lock().await.contains_key(&profile.user_id) {
            self.profiles.lock().await.insert(profile.user_id, profile);
            Ok(())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    async fn delete_profile(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        self.profiles.lock().await.remove(&user_id);
        Ok(())
    }

    async fn profile_exists(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        if *self.should_fail.lock().await {
            return Err(sqlx::Error::RowNotFound);
        }

        *self.find_count.lock().await += 1;
        Ok(self.profiles.lock().await.contains_key(&user_id))
    }
}

pub mod test_helpers {
    use super::*;

    pub fn setup_user_repository() -> MockUserRepository {
        MockUserRepository::new()
    }

    pub fn setup_profile_repository() -> MockUserProfileRepository {
        MockUserProfileRepository::new()
    }

    pub async fn setup_user_with_profile(
        user_repo: &MockUserRepository,
        profile_repo: &MockUserProfileRepository,
        username: &str,
        email: &str,
    ) -> (User, UserProfile) {
        let user_id = Uuid::now_v7();
        let user = fixtures::create_active_user(user_id, username, email);
        let profile = fixtures::create_complete_profile(user_id);

        user_repo.save_user(user.clone()).await.unwrap();
        profile_repo.save_profile(profile.clone()).await.unwrap();

        (user, profile)
    }

    pub fn generate_test_users(count: usize) -> Vec<User> {
        (0..count)
            .map(|i| {
                fixtures::create_test_user(
                    Uuid::now_v7(),
                    &format!("user{}", i),
                    &format!("user{}@example.com", i),
                )
            })
            .collect()
    }

    pub fn generate_test_profiles(user_ids: &[Uuid]) -> Vec<UserProfile> {
        user_ids
            .iter()
            .enumerate()
            .map(|(i, &user_id)| {
                fixtures::create_test_profile(
                    user_id,
                    Some(&format!("First{}", i)),
                    Some(&format!("Last{}", i)),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_repositories_integration() {
        let user_repo = test_helpers::setup_user_repository();
        let profile_repo = test_helpers::setup_profile_repository();

        let (user, profile) = test_helpers::setup_user_with_profile(
            &user_repo,
            &profile_repo,
            "testuser",
            "test@example.com",
        ).await;

        assert_eq!(user_repo.user_count().await, 1);
        assert_eq!(profile_repo.profile_count().await, 1);

        let found_user = user_repo.find_user_by_email("test@example.com").await.unwrap().unwrap();
        let found_profile = profile_repo.find_profile_by_user_id(user.id).await.unwrap().unwrap();

        assert_eq!(found_user.id, user.id);
        assert_eq!(found_profile.user_id, profile.user_id);
    }

    #[tokio::test]
    async fn test_bulk_user_generation() {
        let users = test_helpers::generate_test_users(5);
        assert_eq!(users.len(), 5);

        for (i, user) in users.iter().enumerate() {
            assert_eq!(user.username, format!("user{}", i));
            assert_eq!(user.email, format!("user{}@example.com", i));
        }
    }

    #[tokio::test]
    async fn test_bulk_profile_generation() {
        let user_ids: Vec<Uuid> = (0..3).map(|_| Uuid::now_v7()).collect();
        let profiles = test_helpers::generate_test_profiles(&user_ids);

        assert_eq!(profiles.len(), 3);
        for (i, profile) in profiles.iter().enumerate() {
            assert_eq!(profile.user_id, user_ids[i]);
            assert_eq!(profile.first_name, Some(format!("First{}", i)));
            assert_eq!(profile.last_name, Some(format!("Last{}", i)));
        }
    }

    #[tokio::test]
    async fn test_repository_cleanup() {
        let user_repo = test_helpers::setup_user_repository();
        let profile_repo = test_helpers::setup_profile_repository();

        test_helpers::setup_user_with_profile(&user_repo, &profile_repo, "test", "test@example.com").await;

        assert_eq!(user_repo.user_count().await, 1);
        assert_eq!(profile_repo.profile_count().await, 1);

        user_repo.clear().await;
        profile_repo.clear().await;

        assert_eq!(user_repo.user_count().await, 0);
        assert_eq!(profile_repo.profile_count().await, 0);
    }

    #[test]
    fn test_fixture_creation() {
        let user_id = Uuid::now_v7();

        let pending_user = fixtures::create_test_user(user_id, "test", "test@example.com");
        assert_eq!(pending_user.status, UserStatus::PendingVerification);
        assert!(!pending_user.email_verified);

        let active_user = fixtures::create_active_user(user_id, "test", "test@example.com");
        assert_eq!(active_user.status, UserStatus::Active);
        assert!(active_user.email_verified);

        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));
        assert_eq!(profile.first_name, Some("John".to_string()));
        assert_eq!(profile.last_name, Some("Doe".to_string()));

        let complete_profile = fixtures::create_complete_profile(user_id);
        assert!(complete_profile.first_name.is_some());
        assert!(complete_profile.bio.is_some());
        assert!(complete_profile.website.is_some());
    }
}