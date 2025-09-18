use async_trait::async_trait;
use uuid::Uuid;

use crate::{events::UserCreated, models::User};

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: User) -> Result<(), sqlx::Error>;
    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error>;
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixtures, MockUserRepository};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mock_repository_creation() {
        let repo = MockUserRepository::new();
        assert_eq!(repo.user_count().await, 0);
        assert_eq!(repo.get_save_count().await, 0);
        assert_eq!(repo.get_find_count().await, 0);
        assert!(repo.get_events().await.is_empty());
    }

    #[tokio::test]
    async fn test_save_user_success() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "testuser", "test@example.com");

        let result = repo.save_user(user.clone()).await;
        assert!(result.is_ok());
        assert_eq!(repo.user_count().await, 1);
        assert_eq!(repo.get_save_count().await, 1);
        assert!(repo.has_user_with_email("test@example.com").await);
    }

    #[tokio::test]
    async fn test_save_user_failure() {
        let repo = MockUserRepository::new();
        repo.set_should_fail(true).await;

        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "testuser", "test@example.com");

        let result = repo.save_user(user).await;
        assert!(result.is_err());
        assert_eq!(repo.user_count().await, 0);
        assert_eq!(repo.get_save_count().await, 0);
    }

    #[tokio::test]
    async fn test_find_user_by_id_success() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "testuser", "test@example.com");

        repo.save_user(user.clone()).await.unwrap();

        let result = repo.find_user_by_id(user_id).await;
        assert!(result.is_ok());

        let found_user = result.unwrap();
        assert!(found_user.is_some());

        let found_user = found_user.unwrap();
        assert_eq!(found_user.id, user_id);
        assert_eq!(found_user.username, "testuser");
        assert_eq!(found_user.email, "test@example.com");
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_user_by_id_not_found() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();

        let result = repo.find_user_by_id(user_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_user_by_id_failure() {
        let repo = MockUserRepository::new();
        repo.set_should_fail(true).await;

        let user_id = Uuid::now_v7();
        let result = repo.find_user_by_id(user_id).await;
        assert!(result.is_err());
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_find_user_by_email_success() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "testuser", "test@example.com");

        repo.save_user(user.clone()).await.unwrap();

        let result = repo.find_user_by_email("test@example.com").await;
        assert!(result.is_ok());

        let found_user = result.unwrap();
        assert!(found_user.is_some());

        let found_user = found_user.unwrap();
        assert_eq!(found_user.id, user_id);
        assert_eq!(found_user.email, "test@example.com");
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_user_by_email_not_found() {
        let repo = MockUserRepository::new();

        let result = repo.find_user_by_email("nonexistent@example.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(repo.get_find_count().await, 1);
    }

    #[tokio::test]
    async fn test_find_user_by_email_failure() {
        let repo = MockUserRepository::new();
        repo.set_should_fail(true).await;

        let result = repo.find_user_by_email("test@example.com").await;
        assert!(result.is_err());
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_save_event_success() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let event = fixtures::create_test_event(user_id, "testuser", "test@example.com");

        let result = repo.save_event(event.clone()).await;
        assert!(result.is_ok());

        let events = repo.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, user_id);
        assert_eq!(events[0].username, "testuser");
        assert_eq!(events[0].email, "test@example.com");
    }

    #[tokio::test]
    async fn test_save_event_failure() {
        let repo = MockUserRepository::new();
        repo.set_should_fail(true).await;

        let user_id = Uuid::now_v7();
        let event = fixtures::create_test_event(user_id, "testuser", "test@example.com");

        let result = repo.save_event(event).await;
        assert!(result.is_err());
        assert!(repo.get_events().await.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_users() {
        let repo = MockUserRepository::new();

        let user1_id = Uuid::now_v7();
        let user1 = fixtures::create_test_user(user1_id, "user1", "user1@example.com");

        let user2_id = Uuid::now_v7();
        let user2 = fixtures::create_test_user(user2_id, "user2", "user2@example.com");

        repo.save_user(user1).await.unwrap();
        repo.save_user(user2).await.unwrap();

        assert_eq!(repo.user_count().await, 2);
        assert_eq!(repo.get_save_count().await, 2);

        let found_user1 = repo.find_user_by_id(user1_id).await.unwrap().unwrap();
        let found_user2 = repo.find_user_by_email("user2@example.com").await.unwrap().unwrap();

        assert_eq!(found_user1.username, "user1");
        assert_eq!(found_user2.username, "user2");
        assert_eq!(repo.get_find_count().await, 2);
    }

    #[tokio::test]
    async fn test_duplicate_user_save() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user1 = fixtures::create_test_user(user_id, "testuser", "test@example.com");
        let user2 = fixtures::create_test_user(user_id, "updateduser", "updated@example.com");

        repo.save_user(user1).await.unwrap();
        repo.save_user(user2).await.unwrap();

        // Should overwrite the first user
        assert_eq!(repo.user_count().await, 1);

        let found_user = repo.find_user_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found_user.username, "updateduser");
        assert_eq!(found_user.email, "updated@example.com");
    }

    #[tokio::test]
    async fn test_repository_clear() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "testuser", "test@example.com");
        let event = fixtures::create_test_event(user_id, "testuser", "test@example.com");

        repo.save_user(user).await.unwrap();
        repo.save_event(event).await.unwrap();
        repo.find_user_by_id(user_id).await.unwrap();

        assert_eq!(repo.user_count().await, 1);
        assert_eq!(repo.get_events().await.len(), 1);
        assert_eq!(repo.get_save_count().await, 1);
        assert_eq!(repo.get_find_count().await, 1);

        repo.clear().await;

        assert_eq!(repo.user_count().await, 0);
        assert_eq!(repo.get_events().await.len(), 0);
        assert_eq!(repo.get_save_count().await, 0);
        assert_eq!(repo.get_find_count().await, 0);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let repo = Arc::new(MockUserRepository::new());
        let mut handles = vec![];

        for i in 0..10 {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move {
                let user_id = Uuid::now_v7();
                let user = fixtures::create_test_user(user_id, &format!("user{}", i), &format!("user{}@example.com", i));
                repo_clone.save_user(user).await.unwrap();
                repo_clone.find_user_by_id(user_id).await.unwrap()
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_some());
        }

        assert_eq!(repo.user_count().await, 10);
        assert_eq!(repo.get_save_count().await, 10);
        assert_eq!(repo.get_find_count().await, 10);
    }

    #[tokio::test]
    async fn test_edge_case_empty_strings() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "", "");

        let result = repo.save_user(user).await;
        assert!(result.is_ok());

        let found_user = repo.find_user_by_email("").await.unwrap().unwrap();
        assert_eq!(found_user.username, "");
        assert_eq!(found_user.email, "");
    }

    #[tokio::test]
    async fn test_edge_case_unicode() {
        let repo = MockUserRepository::new();
        let user_id = Uuid::now_v7();
        let user = fixtures::create_test_user(user_id, "测试用户", "测试@example.com");

        repo.save_user(user).await.unwrap();

        let found_user = repo.find_user_by_email("测试@example.com").await.unwrap().unwrap();
        assert_eq!(found_user.username, "测试用户");
    }
}
