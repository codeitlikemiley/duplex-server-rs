use async_trait::async_trait;
use uuid::Uuid;

use crate::models::UserProfile;

#[async_trait]
pub trait UserProfileRepository {
    async fn save_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error>;
    async fn find_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<UserProfile>, sqlx::Error>;
    async fn update_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error>;
    async fn delete_profile(&self, user_id: Uuid) -> Result<(), sqlx::Error>;
    async fn profile_exists(&self, user_id: Uuid) -> Result<bool, sqlx::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixtures, MockUserProfileRepository};

    #[tokio::test]
    async fn test_mock_repository_creation() {
        let repo = MockUserProfileRepository::new();
        assert_eq!(repo.profile_count().await, 0);
        assert_eq!(repo.get_save_count().await, 0);
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_save_profile_success() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        let result = repo.save_profile(profile.clone()).await;
        assert!(result.is_ok());
        assert_eq!(repo.profile_count().await, 1);
        assert_eq!(repo.get_save_count().await, 1);
        assert!(repo.has_profile_for_user(user_id).await);
    }

    #[tokio::test]
    async fn test_save_profile_failure() {
        let repo = MockUserProfileRepository::new();
        repo.set_should_fail(true).await;

        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        let result = repo.save_profile(profile).await;
        assert!(result.is_err());
        assert_eq!(repo.profile_count().await, 0);
        assert_eq!(repo.get_save_count().await, 0);
    }

    #[tokio::test]
    async fn test_find_profile_by_user_id_success() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        repo.save_profile(profile.clone()).await.unwrap();

        let result = repo.find_profile_by_user_id(user_id).await;
        assert!(result.is_ok());

        let found_profile = result.unwrap();
        assert!(found_profile.is_some());

        let found_profile = found_profile.unwrap();
        assert_eq!(found_profile.user_id, user_id);
        assert_eq!(found_profile.first_name, Some("John".to_string()));
        assert_eq!(found_profile.last_name, Some("Doe".to_string()));
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_profile_by_user_id_not_found() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();

        let result = repo.find_profile_by_user_id(user_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_profile_by_user_id_failure() {
        let repo = MockUserProfileRepository::new();
        repo.set_should_fail(true).await;

        let user_id = uuid::Uuid::now_v7();
        let result = repo.find_profile_by_user_id(user_id).await;
        assert!(result.is_err());
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_update_profile_success() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        repo.save_profile(profile.clone()).await.unwrap();

        let mut updated_profile = profile.clone();
        updated_profile.first_name = Some("Jane".to_string());

        let result = repo.update_profile(updated_profile.clone()).await;
        assert!(result.is_ok());

        let found_profile = repo.find_profile_by_user_id(user_id).await.unwrap().unwrap();
        assert_eq!(found_profile.first_name, Some("Jane".to_string()));
    }

    #[tokio::test]
    async fn test_update_profile_not_found() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        let result = repo.update_profile(profile).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_profile_failure() {
        let repo = MockUserProfileRepository::new();
        repo.set_should_fail(true).await;

        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        let result = repo.update_profile(profile).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_profile_success() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        repo.save_profile(profile).await.unwrap();
        assert_eq!(repo.profile_count().await, 1);

        let result = repo.delete_profile(user_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_profile_failure() {
        let repo = MockUserProfileRepository::new();
        repo.set_should_fail(true).await;

        let user_id = uuid::Uuid::now_v7();
        let result = repo.delete_profile(user_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_profile_exists_true() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        repo.save_profile(profile).await.unwrap();

        let result = repo.profile_exists(user_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_profile_exists_false() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();

        let result = repo.profile_exists(user_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_profile_exists_failure() {
        let repo = MockUserProfileRepository::new();
        repo.set_should_fail(true).await;

        let user_id = uuid::Uuid::now_v7();
        let result = repo.profile_exists(user_id).await;
        assert!(result.is_err());
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_multiple_profiles() {
        let repo = MockUserProfileRepository::new();

        let user1_id = uuid::Uuid::now_v7();
        let profile1 = fixtures::create_test_profile(user1_id, Some("John"), Some("Doe"));

        let user2_id = uuid::Uuid::now_v7();
        let profile2 = fixtures::create_test_profile(user2_id, Some("Jane"), Some("Smith"));

        repo.save_profile(profile1).await.unwrap();
        repo.save_profile(profile2).await.unwrap();

        assert_eq!(repo.profile_count().await, 2);
        assert_eq!(repo.get_save_count().await, 2);

        let found_profile1 = repo.find_profile_by_user_id(user1_id).await.unwrap().unwrap();
        let found_profile2 = repo.find_profile_by_user_id(user2_id).await.unwrap().unwrap();

        assert_eq!(found_profile1.first_name, Some("John".to_string()));
        assert_eq!(found_profile2.first_name, Some("Jane".to_string()));
        assert_eq!(repo.get_find_count().await, 2);
    }

    #[tokio::test]
    async fn test_repository_clear() {
        let repo = MockUserProfileRepository::new();
        let user_id = uuid::Uuid::now_v7();
        let profile = fixtures::create_test_profile(user_id, Some("John"), Some("Doe"));

        repo.save_profile(profile).await.unwrap();
        repo.find_profile_by_user_id(user_id).await.unwrap();

        assert_eq!(repo.profile_count().await, 1);
        assert_eq!(repo.get_save_count().await, 1);
        assert_eq!(repo.get_find_count().await, 1);

        repo.clear().await;

        assert_eq!(repo.profile_count().await, 0);
        assert_eq!(repo.get_save_count().await, 0);
        assert_eq!(repo.get_find_count().await, 0);
    }
}