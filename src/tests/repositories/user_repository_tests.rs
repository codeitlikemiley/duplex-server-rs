//! Tests for User Repository Operations
//!
//! This module contains comprehensive tests for user repository operations,
//! including CRUD operations, queries, and database interactions.

#[cfg(test)]
mod user_repository_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use std::collections::HashMap;

    use crate::domain::models::{User, UserStatus};
    use crate::errors::AppError;

    // Mock database for testing
    struct MockUserRepository {
        users: HashMap<Uuid, User>,
        users_by_email: HashMap<String, Uuid>,
        events: Vec<MockUserEvent>,
        fail_on_save: bool,
        fail_on_find: bool,
    }

    #[derive(Debug, Clone)]
    struct MockUserEvent {
        user_id: Uuid,
        event_type: String,
        timestamp: chrono::DateTime<Utc>,
        data: String,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: HashMap::new(),
                users_by_email: HashMap::new(),
                events: Vec::new(),
                fail_on_save: false,
                fail_on_find: false,
            }
        }

        fn with_failure_mode(mut self, save_fails: bool, find_fails: bool) -> Self {
            self.fail_on_save = save_fails;
            self.fail_on_find = find_fails;
            self
        }

        async fn save_user(&mut self, user: User) -> Result<(), AppError> {
            if self.fail_on_save {
                return Err(AppError::Database {
                    message: "Mock save failure".to_string(),
                });
            }

            // Check for duplicate email
            if let Some(existing_id) = self.users_by_email.get(&user.email) {
                if existing_id != &user.id {
                    return Err(AppError::Validation {
                        field: "email".to_string(),
                        message: "Email already exists".to_string(),
                    });
                }
            }

            // Update email index
            self.users_by_email.insert(user.email.clone(), user.id);

            // Save user
            self.users.insert(user.id, user.clone());

            // Record event
            self.events.push(MockUserEvent {
                user_id: user.id,
                event_type: "UserSaved".to_string(),
                timestamp: Utc::now(),
                data: format!("User {} saved", user.username),
            });

            Ok(())
        }

        async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            Ok(self.users.get(&id).cloned())
        }

        async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            if let Some(user_id) = self.users_by_email.get(email) {
                Ok(self.users.get(user_id).cloned())
            } else {
                Ok(None)
            }
        }

        async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            Ok(self.users.values().find(|u| u.username == username).cloned())
        }

        async fn update_user(&mut self, user: User) -> Result<(), AppError> {
            if self.fail_on_save {
                return Err(AppError::Database {
                    message: "Mock update failure".to_string(),
                });
            }

            if !self.users.contains_key(&user.id) {
                return Err(AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(user.id.to_string()),
                });
            }

            // Update email index if email changed
            if let Some(old_user) = self.users.get(&user.id) {
                if old_user.email != user.email {
                    self.users_by_email.remove(&old_user.email);
                    self.users_by_email.insert(user.email.clone(), user.id);
                }
            }

            self.users.insert(user.id, user.clone());

            self.events.push(MockUserEvent {
                user_id: user.id,
                event_type: "UserUpdated".to_string(),
                timestamp: Utc::now(),
                data: format!("User {} updated", user.username),
            });

            Ok(())
        }

        async fn delete_user(&mut self, id: Uuid) -> Result<(), AppError> {
            if self.fail_on_save {
                return Err(AppError::Database {
                    message: "Mock delete failure".to_string(),
                });
            }

            if let Some(user) = self.users.remove(&id) {
                self.users_by_email.remove(&user.email);

                self.events.push(MockUserEvent {
                    user_id: id,
                    event_type: "UserDeleted".to_string(),
                    timestamp: Utc::now(),
                    data: format!("User {} deleted", user.username),
                });

                Ok(())
            } else {
                Err(AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(id.to_string()),
                })
            }
        }

        async fn list_users(&self, limit: usize, offset: usize) -> Result<Vec<User>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock list failure".to_string(),
                });
            }

            let users: Vec<User> = self.users
                .values()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect();

            Ok(users)
        }

        async fn count_users(&self) -> Result<usize, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock count failure".to_string(),
                });
            }

            Ok(self.users.len())
        }

        fn get_events(&self) -> &[MockUserEvent] {
            &self.events
        }
    }

    // Helper function to create a test user
    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            status: UserStatus::Active,
            email_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        }
    }

    #[tokio::test]
    async fn test_save_user() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        // Save user
        let result = repo.save_user(user.clone()).await;
        assert!(result.is_ok());

        // Verify user was saved
        let found = repo.find_user_by_id(user.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);

        // Verify event was recorded
        assert_eq!(repo.get_events().len(), 1);
        assert_eq!(repo.get_events()[0].event_type, "UserSaved");
    }

    #[tokio::test]
    async fn test_find_user_by_id() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        // User not found initially
        let found = repo.find_user_by_id(user.id).await.unwrap();
        assert!(found.is_none());

        // Save user
        repo.save_user(user.clone()).await.unwrap();

        // User found after saving
        let found = repo.find_user_by_id(user.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, user.username);
    }

    #[tokio::test]
    async fn test_find_user_by_email() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        // Save user
        repo.save_user(user.clone()).await.unwrap();

        // Find by email
        let found = repo.find_user_by_email(&user.email).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);

        // Non-existent email
        let not_found = repo.find_user_by_email("nonexistent@example.com").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_user_by_username() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        repo.save_user(user.clone()).await.unwrap();

        // Find by username
        let found = repo.find_user_by_username(&user.username).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, user.email);

        // Non-existent username
        let not_found = repo.find_user_by_username("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_user() {
        let mut repo = MockUserRepository::new();
        let mut user = create_test_user();

        // Save initial user
        repo.save_user(user.clone()).await.unwrap();

        // Update user fields
        user.username = "updateduser".to_string();
        user.email_verified = true;
        user.updated_at = Utc::now();

        let result = repo.update_user(user.clone()).await;
        assert!(result.is_ok());

        // Verify update
        let found = repo.find_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(found.username, "updateduser");
        assert!(found.email_verified);

        // Verify event
        assert_eq!(repo.get_events().len(), 2);
        assert_eq!(repo.get_events()[1].event_type, "UserUpdated");
    }

    #[tokio::test]
    async fn test_update_user_email() {
        let mut repo = MockUserRepository::new();
        let mut user = create_test_user();
        let old_email = user.email.clone();

        repo.save_user(user.clone()).await.unwrap();

        // Update email
        user.email = "newemail@example.com".to_string();
        repo.update_user(user.clone()).await.unwrap();

        // Old email should not find user
        let not_found = repo.find_user_by_email(&old_email).await.unwrap();
        assert!(not_found.is_none());

        // New email should find user
        let found = repo.find_user_by_email(&user.email).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        repo.save_user(user.clone()).await.unwrap();

        // Delete user
        let result = repo.delete_user(user.id).await;
        assert!(result.is_ok());

        // User should not be found
        let found = repo.find_user_by_id(user.id).await.unwrap();
        assert!(found.is_none());

        // Email index should be cleaned
        let found = repo.find_user_by_email(&user.email).await.unwrap();
        assert!(found.is_none());

        // Verify event
        assert_eq!(repo.get_events().len(), 2);
        assert_eq!(repo.get_events()[1].event_type, "UserDeleted");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let mut repo = MockUserRepository::new();
        let random_id = Uuid::new_v4();

        let result = repo.delete_user(random_id).await;
        assert!(result.is_err());

        if let Err(AppError::NotFound { resource, .. }) = result {
            assert_eq!(resource, "User");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[tokio::test]
    async fn test_duplicate_email_validation() {
        let mut repo = MockUserRepository::new();

        let user1 = create_test_user();
        let mut user2 = create_test_user();
        user2.id = Uuid::new_v4();
        user2.username = "anotheruser".to_string();
        // Same email as user1

        // First user saves successfully
        repo.save_user(user1.clone()).await.unwrap();

        // Second user with same email fails
        let result = repo.save_user(user2).await;
        assert!(result.is_err());

        if let Err(AppError::Validation { field, .. }) = result {
            assert_eq!(field, "email");
        } else {
            panic!("Expected Validation error");
        }
    }

    #[tokio::test]
    async fn test_list_users() {
        let mut repo = MockUserRepository::new();

        // Create and save multiple users
        for i in 0..5 {
            let mut user = create_test_user();
            user.id = Uuid::new_v4();
            user.username = format!("user{}", i);
            user.email = format!("user{}@example.com", i);
            repo.save_user(user).await.unwrap();
        }

        // List all users
        let users = repo.list_users(10, 0).await.unwrap();
        assert_eq!(users.len(), 5);

        // List with pagination
        let page1 = repo.list_users(2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = repo.list_users(2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = repo.list_users(2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[tokio::test]
    async fn test_count_users() {
        let mut repo = MockUserRepository::new();

        // Initially empty
        assert_eq!(repo.count_users().await.unwrap(), 0);

        // Add users
        for i in 0..3 {
            let mut user = create_test_user();
            user.id = Uuid::new_v4();
            user.username = format!("user{}", i);
            user.email = format!("user{}@example.com", i);
            repo.save_user(user).await.unwrap();
        }

        assert_eq!(repo.count_users().await.unwrap(), 3);

        // Delete one user
        let users = repo.list_users(1, 0).await.unwrap();
        repo.delete_user(users[0].id).await.unwrap();

        assert_eq!(repo.count_users().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_repository_failure_modes() {
        // Test save failure
        let mut repo = MockUserRepository::new().with_failure_mode(true, false);
        let user = create_test_user();

        let result = repo.save_user(user.clone()).await;
        assert!(result.is_err());

        // Test find failure
        let mut repo = MockUserRepository::new().with_failure_mode(false, true);
        repo.save_user(user.clone()).await.unwrap(); // This works

        let result = repo.find_user_by_id(user.id).await;
        assert!(result.is_err());

        let result = repo.find_user_by_email(&user.email).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_user_status_transitions() {
        let mut repo = MockUserRepository::new();
        let mut user = create_test_user();

        repo.save_user(user.clone()).await.unwrap();

        // Test status transitions
        user.status = UserStatus::Suspended;
        repo.update_user(user.clone()).await.unwrap();

        let found = repo.find_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(found.status, UserStatus::Suspended);

        user.status = UserStatus::Inactive;
        repo.update_user(user.clone()).await.unwrap();

        let found = repo.find_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(found.status, UserStatus::Inactive);
    }

    #[tokio::test]
    async fn test_last_login_tracking() {
        let mut repo = MockUserRepository::new();
        let mut user = create_test_user();

        assert!(user.last_login_at.is_none());
        repo.save_user(user.clone()).await.unwrap();

        // Update last login
        user.last_login_at = Some(Utc::now());
        repo.update_user(user.clone()).await.unwrap();

        let found = repo.find_user_by_id(user.id).await.unwrap().unwrap();
        assert!(found.last_login_at.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        // Save user
        repo.save_user(user.clone()).await.unwrap();

        // Multiple updates (simulating concurrent access)
        let mut user1 = user.clone();
        user1.username = "update1".to_string();

        let mut user2 = user.clone();
        user2.username = "update2".to_string();

        // Both updates succeed (last write wins)
        repo.update_user(user1).await.unwrap();
        repo.update_user(user2.clone()).await.unwrap();

        let found = repo.find_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(found.username, "update2");
    }

    #[tokio::test]
    async fn test_event_ordering() {
        let mut repo = MockUserRepository::new();
        let user = create_test_user();

        // Perform multiple operations
        repo.save_user(user.clone()).await.unwrap();

        let mut updated_user = user.clone();
        updated_user.username = "updated".to_string();
        repo.update_user(updated_user).await.unwrap();

        repo.delete_user(user.id).await.unwrap();

        // Check event order
        let events = repo.get_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "UserSaved");
        assert_eq!(events[1].event_type, "UserUpdated");
        assert_eq!(events[2].event_type, "UserDeleted");

        // Events should be chronologically ordered
        assert!(events[0].timestamp <= events[1].timestamp);
        assert!(events[1].timestamp <= events[2].timestamp);
    }
}