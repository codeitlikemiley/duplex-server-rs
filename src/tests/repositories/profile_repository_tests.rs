//! Tests for Profile Repository Operations
//!
//! This module contains comprehensive tests for user profile repository operations,
//! including CRUD operations, validation, and relationship management.

#[cfg(test)]
mod profile_repository_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use std::collections::HashMap;

    use crate::domain::models::UserProfile;
    use crate::errors::AppError;

    // Mock profile repository for testing
    struct MockProfileRepository {
        profiles: HashMap<Uuid, UserProfile>,
        events: Vec<ProfileEvent>,
        fail_on_save: bool,
        fail_on_find: bool,
        fail_on_delete: bool,
    }

    #[derive(Debug, Clone)]
    struct ProfileEvent {
        user_id: Uuid,
        event_type: String,
        timestamp: chrono::DateTime<Utc>,
        changes: Vec<String>,
    }

    impl MockProfileRepository {
        fn new() -> Self {
            Self {
                profiles: HashMap::new(),
                events: Vec::new(),
                fail_on_save: false,
                fail_on_find: false,
                fail_on_delete: false,
            }
        }

        fn with_failure_mode(mut self, save: bool, find: bool, delete: bool) -> Self {
            self.fail_on_save = save;
            self.fail_on_find = find;
            self.fail_on_delete = delete;
            self
        }

        async fn save_profile(&mut self, profile: UserProfile) -> Result<(), AppError> {
            if self.fail_on_save {
                return Err(AppError::Database {
                    message: "Mock save failure".to_string(),
                });
            }

            let is_update = self.profiles.contains_key(&profile.user_id);
            let event_type = if is_update { "ProfileUpdated" } else { "ProfileCreated" };

            // Track changes if updating
            let mut changes = Vec::new();
            if let Some(existing) = self.profiles.get(&profile.user_id) {
                if existing.first_name != profile.first_name {
                    changes.push("first_name".to_string());
                }
                if existing.last_name != profile.last_name {
                    changes.push("last_name".to_string());
                }
                if existing.bio != profile.bio {
                    changes.push("bio".to_string());
                }
                if existing.avatar_url != profile.avatar_url {
                    changes.push("avatar_url".to_string());
                }
                if existing.website != profile.website {
                    changes.push("website".to_string());
                }
                if existing.location != profile.location {
                    changes.push("location".to_string());
                }
                if existing.preferences != profile.preferences {
                    changes.push("preferences".to_string());
                }
            }

            self.profiles.insert(profile.user_id, profile.clone());

            self.events.push(ProfileEvent {
                user_id: profile.user_id,
                event_type: event_type.to_string(),
                timestamp: Utc::now(),
                changes,
            });

            Ok(())
        }

        async fn find_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<UserProfile>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            Ok(self.profiles.get(&user_id).cloned())
        }

        async fn update_profile(&mut self, profile: UserProfile) -> Result<(), AppError> {
            if !self.profiles.contains_key(&profile.user_id) {
                return Err(AppError::NotFound {
                    resource: "UserProfile".to_string(),
                    id: Some(profile.user_id.to_string()),
                });
            }

            self.save_profile(profile).await
        }

        async fn delete_profile(&mut self, user_id: Uuid) -> Result<(), AppError> {
            if self.fail_on_delete {
                return Err(AppError::Database {
                    message: "Mock delete failure".to_string(),
                });
            }

            if self.profiles.remove(&user_id).is_some() {
                self.events.push(ProfileEvent {
                    user_id,
                    event_type: "ProfileDeleted".to_string(),
                    timestamp: Utc::now(),
                    changes: vec![],
                });
                Ok(())
            } else {
                Err(AppError::NotFound {
                    resource: "UserProfile".to_string(),
                    id: Some(user_id.to_string()),
                })
            }
        }

        async fn find_profiles_by_location(&self, location: &str) -> Result<Vec<UserProfile>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            let profiles: Vec<UserProfile> = self.profiles
                .values()
                .filter(|p| p.location.as_deref() == Some(location))
                .cloned()
                .collect();

            Ok(profiles)
        }

        async fn find_incomplete_profiles(&self) -> Result<Vec<UserProfile>, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock find failure".to_string(),
                });
            }

            let profiles: Vec<UserProfile> = self.profiles
                .values()
                .filter(|p| {
                    p.first_name.is_none() ||
                    p.last_name.is_none() ||
                    p.bio.is_none() ||
                    p.avatar_url.is_none()
                })
                .cloned()
                .collect();

            Ok(profiles)
        }

        async fn count_profiles(&self) -> Result<usize, AppError> {
            if self.fail_on_find {
                return Err(AppError::Database {
                    message: "Mock count failure".to_string(),
                });
            }

            Ok(self.profiles.len())
        }

        async fn bulk_update_preferences(&mut self, user_ids: Vec<Uuid>, preferences: serde_json::Value) -> Result<usize, AppError> {
            if self.fail_on_save {
                return Err(AppError::Database {
                    message: "Mock bulk update failure".to_string(),
                });
            }

            let mut updated_count = 0;
            for user_id in user_ids {
                if let Some(profile) = self.profiles.get_mut(&user_id) {
                    profile.preferences = preferences.clone();
                    profile.updated_at = Utc::now();
                    updated_count += 1;

                    self.events.push(ProfileEvent {
                        user_id,
                        event_type: "PreferencesUpdated".to_string(),
                        timestamp: Utc::now(),
                        changes: vec!["preferences".to_string()],
                    });
                }
            }

            Ok(updated_count)
        }

        fn get_events(&self) -> &[ProfileEvent] {
            &self.events
        }
    }

    // Helper function to create a test profile
    fn create_test_profile(user_id: Uuid) -> UserProfile {
        UserProfile {
            user_id,
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            bio: Some("Software developer".to_string()),
            website: Some("https://johndoe.com".to_string()),
            location: Some("San Francisco, CA".to_string()),
            preferences: json!({
                "theme": "dark",
                "notifications": true,
                "language": "en"
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_save_profile() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let profile = create_test_profile(user_id);

        // Save profile
        let result = repo.save_profile(profile.clone()).await;
        assert!(result.is_ok());

        // Verify profile was saved
        let found = repo.find_profile_by_user_id(user_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().first_name, profile.first_name);

        // Verify event was recorded
        assert_eq!(repo.get_events().len(), 1);
        assert_eq!(repo.get_events()[0].event_type, "ProfileCreated");
    }

    #[tokio::test]
    async fn test_find_profile_by_user_id() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let profile = create_test_profile(user_id);

        // Profile not found initially
        let found = repo.find_profile_by_user_id(user_id).await.unwrap();
        assert!(found.is_none());

        // Save profile
        repo.save_profile(profile.clone()).await.unwrap();

        // Profile found after saving
        let found = repo.find_profile_by_user_id(user_id).await.unwrap();
        assert!(found.is_some());

        let found_profile = found.unwrap();
        assert_eq!(found_profile.bio, profile.bio);
        assert_eq!(found_profile.website, profile.website);
    }

    #[tokio::test]
    async fn test_update_profile() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let mut profile = create_test_profile(user_id);

        // Save initial profile
        repo.save_profile(profile.clone()).await.unwrap();

        // Update profile fields
        profile.first_name = Some("Jane".to_string());
        profile.bio = Some("Senior Software Engineer".to_string());
        profile.location = Some("New York, NY".to_string());
        profile.updated_at = Utc::now();

        let result = repo.update_profile(profile.clone()).await;
        assert!(result.is_ok());

        // Verify updates
        let found = repo.find_profile_by_user_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.first_name, Some("Jane".to_string()));
        assert_eq!(found.bio, Some("Senior Software Engineer".to_string()));
        assert_eq!(found.location, Some("New York, NY".to_string()));

        // Check events
        assert_eq!(repo.get_events().len(), 2);
        assert_eq!(repo.get_events()[1].event_type, "ProfileUpdated");
        assert!(repo.get_events()[1].changes.contains(&"first_name".to_string()));
        assert!(repo.get_events()[1].changes.contains(&"bio".to_string()));
        assert!(repo.get_events()[1].changes.contains(&"location".to_string()));
    }

    #[tokio::test]
    async fn test_update_nonexistent_profile() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let profile = create_test_profile(user_id);

        // Try to update non-existent profile
        let result = repo.update_profile(profile).await;
        assert!(result.is_err());

        if let Err(AppError::NotFound { resource, .. }) = result {
            assert_eq!(resource, "UserProfile");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let profile = create_test_profile(user_id);

        // Save profile
        repo.save_profile(profile).await.unwrap();

        // Delete profile
        let result = repo.delete_profile(user_id).await;
        assert!(result.is_ok());

        // Profile should not be found
        let found = repo.find_profile_by_user_id(user_id).await.unwrap();
        assert!(found.is_none());

        // Check events
        assert_eq!(repo.get_events().len(), 2);
        assert_eq!(repo.get_events()[1].event_type, "ProfileDeleted");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_profile() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();

        let result = repo.delete_profile(user_id).await;
        assert!(result.is_err());

        if let Err(AppError::NotFound { resource, .. }) = result {
            assert_eq!(resource, "UserProfile");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[tokio::test]
    async fn test_find_profiles_by_location() {
        let mut repo = MockProfileRepository::new();

        // Create profiles with different locations
        let locations = vec![
            ("San Francisco, CA", 3),
            ("New York, NY", 2),
            ("Austin, TX", 1),
        ];

        for (location, count) in &locations {
            for _ in 0..*count {
                let mut profile = create_test_profile(Uuid::new_v4());
                profile.location = Some(location.to_string());
                repo.save_profile(profile).await.unwrap();
            }
        }

        // Find profiles by location
        let sf_profiles = repo.find_profiles_by_location("San Francisco, CA").await.unwrap();
        assert_eq!(sf_profiles.len(), 3);

        let ny_profiles = repo.find_profiles_by_location("New York, NY").await.unwrap();
        assert_eq!(ny_profiles.len(), 2);

        let tx_profiles = repo.find_profiles_by_location("Austin, TX").await.unwrap();
        assert_eq!(tx_profiles.len(), 1);

        // Non-existent location
        let no_profiles = repo.find_profiles_by_location("Chicago, IL").await.unwrap();
        assert_eq!(no_profiles.len(), 0);
    }

    #[tokio::test]
    async fn test_find_incomplete_profiles() {
        let mut repo = MockProfileRepository::new();

        // Complete profile
        let complete_profile = create_test_profile(Uuid::new_v4());
        repo.save_profile(complete_profile).await.unwrap();

        // Incomplete profiles
        let mut incomplete1 = create_test_profile(Uuid::new_v4());
        incomplete1.first_name = None;
        repo.save_profile(incomplete1).await.unwrap();

        let mut incomplete2 = create_test_profile(Uuid::new_v4());
        incomplete2.bio = None;
        repo.save_profile(incomplete2).await.unwrap();

        let mut incomplete3 = create_test_profile(Uuid::new_v4());
        incomplete3.avatar_url = None;
        repo.save_profile(incomplete3).await.unwrap();

        // Find incomplete profiles
        let incomplete = repo.find_incomplete_profiles().await.unwrap();
        assert_eq!(incomplete.len(), 3);
    }

    #[tokio::test]
    async fn test_profile_preferences() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let mut profile = create_test_profile(user_id);

        // Set initial preferences
        profile.preferences = json!({
            "theme": "light",
            "notifications": false,
            "language": "es"
        });

        repo.save_profile(profile.clone()).await.unwrap();

        // Update preferences
        profile.preferences = json!({
            "theme": "dark",
            "notifications": true,
            "language": "en",
            "timezone": "UTC"
        });

        repo.update_profile(profile.clone()).await.unwrap();

        // Verify preferences were updated
        let found = repo.find_profile_by_user_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.preferences["theme"], "dark");
        assert_eq!(found.preferences["notifications"], true);
        assert_eq!(found.preferences["language"], "en");
        assert_eq!(found.preferences["timezone"], "UTC");
    }

    #[tokio::test]
    async fn test_bulk_update_preferences() {
        let mut repo = MockProfileRepository::new();

        // Create multiple profiles
        let mut user_ids = Vec::new();
        for _ in 0..5 {
            let id = Uuid::new_v4();
            let profile = create_test_profile(id);
            repo.save_profile(profile).await.unwrap();
            user_ids.push(id);
        }

        // Bulk update preferences
        let new_preferences = json!({
            "maintenance_mode": true,
            "feature_flags": {
                "new_ui": true,
                "beta_features": false
            }
        });

        let updated = repo.bulk_update_preferences(user_ids.clone(), new_preferences.clone()).await.unwrap();
        assert_eq!(updated, 5);

        // Verify all profiles were updated
        for user_id in user_ids {
            let profile = repo.find_profile_by_user_id(user_id).await.unwrap().unwrap();
            assert_eq!(profile.preferences, new_preferences);
        }

        // Check events
        let pref_events: Vec<_> = repo.get_events()
            .iter()
            .filter(|e| e.event_type == "PreferencesUpdated")
            .collect();
        assert_eq!(pref_events.len(), 5);
    }

    #[tokio::test]
    async fn test_count_profiles() {
        let mut repo = MockProfileRepository::new();

        // Initially empty
        assert_eq!(repo.count_profiles().await.unwrap(), 0);

        // Add profiles
        for i in 0..3 {
            let profile = create_test_profile(Uuid::new_v4());
            repo.save_profile(profile).await.unwrap();
            assert_eq!(repo.count_profiles().await.unwrap(), i + 1);
        }

        // Delete one profile
        let profile_to_delete = repo.profiles.keys().next().cloned().unwrap();
        repo.delete_profile(profile_to_delete).await.unwrap();
        assert_eq!(repo.count_profiles().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_repository_failure_modes() {
        // Test save failure
        let mut repo = MockProfileRepository::new().with_failure_mode(true, false, false);
        let profile = create_test_profile(Uuid::new_v4());

        let result = repo.save_profile(profile.clone()).await;
        assert!(result.is_err());

        // Test find failure
        let mut repo = MockProfileRepository::new().with_failure_mode(false, true, false);
        repo.save_profile(profile.clone()).await.unwrap();

        let result = repo.find_profile_by_user_id(profile.user_id).await;
        assert!(result.is_err());

        // Test delete failure
        let mut repo = MockProfileRepository::new().with_failure_mode(false, false, true);
        repo.save_profile(profile.clone()).await.unwrap();

        let result = repo.delete_profile(profile.user_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_profile_field_validation() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();

        // Test with empty optional fields
        let minimal_profile = UserProfile {
            user_id,
            first_name: None,
            last_name: None,
            avatar_url: None,
            bio: None,
            website: None,
            location: None,
            preferences: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = repo.save_profile(minimal_profile.clone()).await;
        assert!(result.is_ok());

        // Verify minimal profile was saved
        let found = repo.find_profile_by_user_id(user_id).await.unwrap();
        assert!(found.is_some());

        let found_profile = found.unwrap();
        assert!(found_profile.first_name.is_none());
        assert!(found_profile.last_name.is_none());
        assert!(found_profile.bio.is_none());
    }

    #[tokio::test]
    async fn test_profile_timestamps() {
        let mut repo = MockProfileRepository::new();
        let user_id = Uuid::new_v4();
        let profile = create_test_profile(user_id);

        let created_at = profile.created_at;
        repo.save_profile(profile.clone()).await.unwrap();

        // Wait a bit and update
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut updated_profile = profile.clone();
        updated_profile.first_name = Some("UpdatedName".to_string());
        updated_profile.updated_at = Utc::now();

        repo.update_profile(updated_profile.clone()).await.unwrap();

        let found = repo.find_profile_by_user_id(user_id).await.unwrap().unwrap();

        // Created at should remain the same
        assert_eq!(found.created_at, created_at);

        // Updated at should be newer
        assert!(found.updated_at > created_at);
    }

    #[tokio::test]
    async fn test_profile_completeness_calculation() {
        let user_id = Uuid::new_v4();

        // Test various levels of completeness
        let test_cases = vec![
            (UserProfile {
                user_id,
                first_name: None,
                last_name: None,
                avatar_url: None,
                bio: None,
                website: None,
                location: None,
                preferences: json!({}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }, 0), // 0% complete

            (UserProfile {
                user_id,
                first_name: Some("John".to_string()),
                last_name: Some("Doe".to_string()),
                avatar_url: None,
                bio: None,
                website: None,
                location: None,
                preferences: json!({}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }, 2), // 2 fields filled

            (create_test_profile(user_id), 6), // All 6 optional fields filled
        ];

        for (profile, expected_filled) in test_cases {
            let filled_count = vec![
                profile.first_name.is_some(),
                profile.last_name.is_some(),
                profile.avatar_url.is_some(),
                profile.bio.is_some(),
                profile.website.is_some(),
                profile.location.is_some(),
            ].iter().filter(|&&x| x).count();

            assert_eq!(filled_count, expected_filled);
        }
    }
}