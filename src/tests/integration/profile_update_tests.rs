//! Integration Tests for Profile Update Operations
//!
//! This module contains comprehensive integration tests for user profile management.
//! These tests simulate the complete profile update flow including advanced validation,
//! security features, performance optimizations, and real-time collaboration scenarios.

#[cfg(test)]
mod profile_update_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use serde::Serialize;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    #[derive(Debug, Clone, Serialize)]
    struct UserProfile {
        user_id: Uuid,
        first_name: Option<String>,
        last_name: Option<String>,
        bio: Option<String>,
        location: Option<String>,
        website: Option<String>,
        avatar_url: Option<String>,
        preferences_json: Option<String>,
        created_at: chrono::DateTime<Utc>,
        updated_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize)]
    struct ProfileUpdateEvent {
        id: Uuid,
        user_id: Uuid,
        field_name: String,
        old_value: Option<String>,
        new_value: Option<String>,
        updated_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize)]
    struct ActivityLog {
        user_id: Uuid,
        action: String,
        details: String,
        timestamp: chrono::DateTime<Utc>,
        ip_address: String,
        user_agent: String,
    }

    #[derive(Debug, Clone, Serialize)]
    struct ProfileVersion {
        version_id: Uuid,
        user_id: Uuid,
        profile_snapshot: UserProfile,
        created_at: chrono::DateTime<Utc>,
        created_by: Uuid,
        change_reason: String,
    }

    #[derive(Debug, Clone)]
    struct ProfilePermission {
        user_id: Uuid,
        field_name: String,
        can_view: bool,
        can_edit: bool,
        granted_by: Uuid,
        granted_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    struct ProfileNotification {
        id: Uuid,
        recipient_id: Uuid,
        profile_owner_id: Uuid,
        notification_type: NotificationType,
        message: String,
        read: bool,
        created_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum NotificationType {
        ProfileViewed,
        FieldUpdated,
        PermissionGranted,
        PermissionRevoked,
        ProfileCompleted,
    }

    #[derive(Debug, Clone, Serialize)]
    struct ProfileAnalytics {
        user_id: Uuid,
        view_count: u32,
        unique_viewers: HashSet<Uuid>,
        last_viewed: Option<chrono::DateTime<Utc>>,
        update_frequency: HashMap<String, u32>, // field_name -> update count
        completeness_history: Vec<(chrono::DateTime<Utc>, f32)>, // timestamp -> completeness %
    }

    #[derive(Debug, Clone)]
    struct CacheEntry {
        profile: UserProfile,
        cached_at: chrono::DateTime<Utc>,
        ttl_seconds: u64,
    }

    #[derive(Debug, Clone)]
    struct ValidationRule {
        field_name: String,
        rule_type: ValidationType,
        rule_value: String,
        error_message: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum ValidationType {
        MinLength,
        MaxLength,
        Regex,
        Blacklist,
        Custom,
    }

    // Enhanced profile update simulator with advanced features
    struct ProfileUpdateSimulator {
        users: Vec<User>,
        profiles: Vec<UserProfile>,
        update_events: Vec<ProfileUpdateEvent>,
        activity_logs: Vec<ActivityLog>,
        profile_versions: Vec<ProfileVersion>,
        profile_permissions: Vec<ProfilePermission>,
        notifications: Vec<ProfileNotification>,
        analytics: HashMap<Uuid, ProfileAnalytics>,
        profile_cache: HashMap<Uuid, CacheEntry>,
        validation_rules: Vec<ValidationRule>,
        rate_limits: HashMap<String, VecDeque<chrono::DateTime<Utc>>>, // IP -> timestamps
        blocked_words: HashSet<String>,
        moderator_flags: HashMap<Uuid, Vec<String>>, // user_id -> flags
        collaboration_sessions: HashMap<Uuid, CollaborationSession>,
        real_time_watchers: HashMap<Uuid, HashSet<Uuid>>, // profile_id -> watcher_ids
    }

    #[derive(Debug, Clone)]
    struct CollaborationSession {
        session_id: Uuid,
        profile_id: Uuid,
        participants: HashSet<Uuid>,
        active_editors: HashMap<String, Uuid>, // field_name -> editor_id
        pending_changes: HashMap<String, PendingChange>,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    struct PendingChange {
        field_name: String,
        new_value: Option<String>,
        proposed_by: Uuid,
        proposed_at: chrono::DateTime<Utc>,
        approvals: HashSet<Uuid>,
        rejections: HashSet<Uuid>,
    }

    impl ProfileUpdateSimulator {
        fn new() -> Self {
            let mut blocked_words = HashSet::new();
            blocked_words.insert("spam".to_string());
            blocked_words.insert("scam".to_string());
            blocked_words.insert("fraud".to_string());

            Self {
                users: Vec::new(),
                profiles: Vec::new(),
                update_events: Vec::new(),
                activity_logs: Vec::new(),
                profile_versions: Vec::new(),
                profile_permissions: Vec::new(),
                notifications: Vec::new(),
                analytics: HashMap::new(),
                profile_cache: HashMap::new(),
                validation_rules: Vec::new(),
                rate_limits: HashMap::new(),
                blocked_words,
                moderator_flags: HashMap::new(),
                collaboration_sessions: HashMap::new(),
                real_time_watchers: HashMap::new(),
            }
        }

        fn create_test_user_with_profile(&mut self, email: String, username: String) -> (User, UserProfile) {
            let user = User {
                id: Uuid::new_v4(),
                username,
                email: email.clone(),
                password_hash: "hashed_password".to_string(),
                email_verified: true,
                status: UserStatus::Active,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: Some(Utc::now()),
            };

            let profile = UserProfile {
                user_id: user.id,
                first_name: None,
                last_name: None,
                bio: None,
                location: None,
                website: None,
                avatar_url: None,
                preferences_json: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            self.users.push(user.clone());
            self.profiles.push(profile.clone());

            (user, profile)
        }

        fn get_user_profile(&self, user_id: &Uuid) -> Result<UserProfile, AppError> {
            self.profiles.iter()
                .find(|p| p.user_id == *user_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound {
                    resource: "profile".to_string(),
                    id: Some(user_id.to_string()),
                })
        }

        fn validate_profile_field(&self, field_name: &str, value: &str) -> Result<(), AppError> {
            match field_name {
                "first_name" | "last_name" => {
                    if value.len() > 50 {
                        return Err(AppError::Validation {
                            field: field_name.to_string(),
                            message: "Name must be 50 characters or less".to_string(),
                        });
                    }
                }
                "bio" => {
                    if value.len() > 500 {
                        return Err(AppError::Validation {
                            field: "bio".to_string(),
                            message: "Bio must be 500 characters or less".to_string(),
                        });
                    }
                }
                "location" => {
                    if value.len() > 100 {
                        return Err(AppError::Validation {
                            field: "location".to_string(),
                            message: "Location must be 100 characters or less".to_string(),
                        });
                    }
                }
                "website" => {
                    if !value.starts_with("http://") && !value.starts_with("https://") {
                        return Err(AppError::Validation {
                            field: "website".to_string(),
                            message: "Website must be a valid URL starting with http:// or https://".to_string(),
                        });
                    }
                    if value.len() > 200 {
                        return Err(AppError::Validation {
                            field: "website".to_string(),
                            message: "Website URL must be 200 characters or less".to_string(),
                        });
                    }
                }
                "avatar_url" => {
                    if !value.starts_with("http://") && !value.starts_with("https://") {
                        return Err(AppError::Validation {
                            field: "avatar_url".to_string(),
                            message: "Avatar URL must be a valid URL starting with http:// or https://".to_string(),
                        });
                    }
                    if value.len() > 500 {
                        return Err(AppError::Validation {
                            field: "avatar_url".to_string(),
                            message: "Avatar URL must be 500 characters or less".to_string(),
                        });
                    }
                }
                _ => {}
            }
            Ok(())
        }

        fn validate_preferences_json(&self, preferences: &str) -> Result<(), AppError> {
            // Validate JSON format
            serde_json::from_str::<serde_json::Value>(preferences)
                .map_err(|_| AppError::Validation {
                    field: "preferences_json".to_string(),
                    message: "Preferences must be valid JSON".to_string(),
                })?;

            // Size limit check
            if preferences.len() > 5000 {
                return Err(AppError::Validation {
                    field: "preferences_json".to_string(),
                    message: "Preferences JSON must be 5000 characters or less".to_string(),
                });
            }

            Ok(())
        }

        fn update_profile_field(&mut self, user_id: &Uuid, field_name: &str, new_value: Option<String>) -> Result<(), AppError> {
            // Validate user exists and is active
            let user = self.users.iter()
                .find(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            if user.status != UserStatus::Active {
                return Err(AppError::Validation {
                    field: "user".to_string(),
                    message: "Cannot update profile for inactive user".to_string(),
                });
            }

            // Get current profile
            let profile_index = self.profiles.iter()
                .position(|p| p.user_id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "profile".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            let mut profile = self.profiles[profile_index].clone();

            // Store old value for event logging
            let old_value = match field_name {
                "first_name" => profile.first_name.clone(),
                "last_name" => profile.last_name.clone(),
                "bio" => profile.bio.clone(),
                "location" => profile.location.clone(),
                "website" => profile.website.clone(),
                "avatar_url" => profile.avatar_url.clone(),
                "preferences_json" => profile.preferences_json.clone(),
                _ => None,
            };

            // Validate new value if provided
            if let Some(ref value) = new_value {
                if field_name == "preferences_json" {
                    self.validate_preferences_json(value)?;
                } else {
                    self.validate_profile_field(field_name, value)?;
                }
            }

            // Update field
            match field_name {
                "first_name" => profile.first_name = new_value.clone(),
                "last_name" => profile.last_name = new_value.clone(),
                "bio" => profile.bio = new_value.clone(),
                "location" => profile.location = new_value.clone(),
                "website" => profile.website = new_value.clone(),
                "avatar_url" => profile.avatar_url = new_value.clone(),
                "preferences_json" => profile.preferences_json = new_value.clone(),
                _ => return Err(AppError::Validation {
                    field: "field_name".to_string(),
                    message: format!("Unknown profile field: {}", field_name),
                }),
            }

            profile.updated_at = Utc::now();

            // Record update event
            let event = ProfileUpdateEvent {
                id: Uuid::new_v4(),
                user_id: *user_id,
                field_name: field_name.to_string(),
                old_value: old_value.clone(),
                new_value: new_value.clone(),
                updated_at: profile.updated_at,
            };

            self.update_events.push(event);

            // Update profile in storage
            self.profiles[profile_index] = profile;

            // Log activity
            let activity = ActivityLog {
                user_id: *user_id,
                action: "profile_update".to_string(),
                details: format!("Updated {} from {:?} to {:?}", field_name, old_value, new_value),
                timestamp: Utc::now(),
                ip_address: "127.0.0.1".to_string(), // Default IP for tests
                user_agent: "TestAgent/1.0".to_string(),
            };
            self.activity_logs.push(activity);

            Ok(())
        }

        fn bulk_update_profile(&mut self, user_id: &Uuid, updates: Vec<(String, Option<String>)>) -> Result<UserProfile, AppError> {
            // Validate all updates first
            for (field_name, new_value) in &updates {
                if let Some(value) = new_value {
                    if field_name == "preferences_json" {
                        self.validate_preferences_json(value)?;
                    } else {
                        self.validate_profile_field(field_name, value)?;
                    }
                }
            }

            // Apply all updates
            for (field_name, new_value) in updates {
                self.update_profile_field(user_id, &field_name, new_value)?;
            }

            self.get_user_profile(user_id)
        }

        fn get_profile_update_history(&self, user_id: &Uuid) -> Vec<ProfileUpdateEvent> {
            self.update_events.iter()
                .filter(|event| event.user_id == *user_id)
                .cloned()
                .collect()
        }

        fn get_activity_logs(&self, user_id: &Uuid) -> Vec<ActivityLog> {
            self.activity_logs.iter()
                .filter(|log| log.user_id == *user_id)
                .cloned()
                .collect()
        }

        fn delete_profile_field(&mut self, user_id: &Uuid, field_name: &str) -> Result<(), AppError> {
            self.update_profile_field(user_id, field_name, None)
        }

        fn get_profile_completeness(&self, user_id: &Uuid) -> Result<f32, AppError> {
            let profile = self.get_user_profile(user_id)?;

            let total_fields = 7.0; // first_name, last_name, bio, location, website, avatar_url, preferences_json
            let mut completed_fields = 0.0;

            if profile.first_name.is_some() { completed_fields += 1.0; }
            if profile.last_name.is_some() { completed_fields += 1.0; }
            if profile.bio.is_some() { completed_fields += 1.0; }
            if profile.location.is_some() { completed_fields += 1.0; }
            if profile.website.is_some() { completed_fields += 1.0; }
            if profile.avatar_url.is_some() { completed_fields += 1.0; }
            if profile.preferences_json.is_some() { completed_fields += 1.0; }

            Ok((completed_fields / total_fields) * 100.0)
        }

        fn search_profiles_by_field(&self, field_name: &str, search_term: &str) -> Vec<UserProfile> {
            self.profiles.iter()
                .filter(|profile| {
                    match field_name {
                        "first_name" => profile.first_name.as_ref().map_or(false, |name| name.to_lowercase().contains(&search_term.to_lowercase())),
                        "last_name" => profile.last_name.as_ref().map_or(false, |name| name.to_lowercase().contains(&search_term.to_lowercase())),
                        "location" => profile.location.as_ref().map_or(false, |loc| loc.to_lowercase().contains(&search_term.to_lowercase())),
                        "bio" => profile.bio.as_ref().map_or(false, |bio| bio.to_lowercase().contains(&search_term.to_lowercase())),
                        _ => false,
                    }
                })
                .cloned()
                .collect()
        }

        fn validate_profile_privacy(&self, user_id: &Uuid, requesting_user_id: &Uuid) -> Result<bool, AppError> {
            // Basic privacy check - users can always access their own profile
            if user_id == requesting_user_id {
                return Ok(true);
            }

            // For now, all profiles are public
            // In a real system, this would check privacy settings
            Ok(true)
        }

        fn enhanced_update_profile_field(&mut self, user_id: &Uuid, field_name: &str, new_value: Option<String>, ip: &str, user_agent: &str) -> Result<(), AppError> {
            // Check rate limiting
            self.check_update_rate_limit(ip)?;

            // Content moderation
            if let Some(ref value) = new_value {
                self.check_content_moderation(value)?;
            }

            // Create profile version before update
            self.create_profile_version(user_id, "User update".to_string())?;

            // Perform the update
            self.update_profile_field(user_id, field_name, new_value.clone())?;

            // Update analytics
            self.update_analytics(user_id, field_name);

            // Update cache
            self.invalidate_profile_cache(user_id);

            // Send notifications to watchers
            self.notify_profile_watchers(user_id, field_name, &new_value);

            Ok(())
        }

        fn check_update_rate_limit(&mut self, ip: &str) -> Result<(), AppError> {
            let now = Utc::now();
            let window_start = now - Duration::minutes(5);

            let attempts = self.rate_limits.entry(ip.to_string()).or_insert(VecDeque::new());

            // Remove old attempts
            while let Some(&front_time) = attempts.front() {
                if front_time < window_start {
                    attempts.pop_front();
                } else {
                    break;
                }
            }

            // Check rate limit (20 updates per 5 minutes)
            if attempts.len() >= 20 {
                return Err(AppError::Authentication {
                    message: "Too many profile updates. Please slow down.".to_string(),
                });
            }

            attempts.push_back(now);
            Ok(())
        }

        fn check_content_moderation(&self, content: &str) -> Result<(), AppError> {
            let content_lower = content.to_lowercase();

            // Check for blocked words
            for blocked_word in &self.blocked_words {
                if content_lower.contains(blocked_word) {
                    return Err(AppError::Validation {
                        field: "content".to_string(),
                        message: format!("Content contains prohibited word: {}", blocked_word),
                    });
                }
            }

            // Check for excessive capitalization
            let caps_count = content.chars().filter(|c| c.is_uppercase()).count();
            let total_letters = content.chars().filter(|c| c.is_alphabetic()).count();

            if total_letters > 0 && (caps_count as f64 / total_letters as f64) > 0.7 {
                return Err(AppError::Validation {
                    field: "content".to_string(),
                    message: "Please reduce the use of capital letters".to_string(),
                });
            }

            Ok(())
        }

        fn create_profile_version(&mut self, user_id: &Uuid, reason: String) -> Result<(), AppError> {
            let profile = self.get_user_profile(user_id)?;

            let version = ProfileVersion {
                version_id: Uuid::new_v4(),
                user_id: *user_id,
                profile_snapshot: profile,
                created_at: Utc::now(),
                created_by: *user_id,
                change_reason: reason,
            };

            self.profile_versions.push(version);
            Ok(())
        }

        fn update_analytics(&mut self, user_id: &Uuid, field_name: &str) {
            // Get completeness before borrowing analytics mutably
            let completeness = self.get_profile_completeness(user_id).unwrap_or(0.0);

            let analytics = self.analytics.entry(*user_id).or_insert(ProfileAnalytics {
                user_id: *user_id,
                view_count: 0,
                unique_viewers: HashSet::new(),
                last_viewed: None,
                update_frequency: HashMap::new(),
                completeness_history: Vec::new(),
            });

            // Update field frequency
            let count = analytics.update_frequency.entry(field_name.to_string()).or_insert(0);
            *count += 1;

            // Update completeness history
            analytics.completeness_history.push((Utc::now(), completeness));
        }

        fn invalidate_profile_cache(&mut self, user_id: &Uuid) {
            self.profile_cache.remove(user_id);
        }

        fn get_cached_profile(&mut self, user_id: &Uuid) -> Option<UserProfile> {
            if let Some(cache_entry) = self.profile_cache.get(user_id) {
                let now = Utc::now();
                let cache_age = (now - cache_entry.cached_at).num_seconds() as u64;

                if cache_age < cache_entry.ttl_seconds {
                    return Some(cache_entry.profile.clone());
                } else {
                    self.profile_cache.remove(user_id);
                }
            }
            None
        }

        fn cache_profile(&mut self, user_id: &Uuid, profile: &UserProfile, ttl_seconds: u64) {
            let cache_entry = CacheEntry {
                profile: profile.clone(),
                cached_at: Utc::now(),
                ttl_seconds,
            };
            self.profile_cache.insert(*user_id, cache_entry);
        }

        fn notify_profile_watchers(&mut self, user_id: &Uuid, field_name: &str, new_value: &Option<String>) {
            if let Some(watchers) = self.real_time_watchers.get(user_id) {
                for watcher_id in watchers {
                    let notification = ProfileNotification {
                        id: Uuid::new_v4(),
                        recipient_id: *watcher_id,
                        profile_owner_id: *user_id,
                        notification_type: NotificationType::FieldUpdated,
                        message: format!("Profile field '{}' was updated", field_name),
                        read: false,
                        created_at: Utc::now(),
                    };
                    self.notifications.push(notification);
                }
            }
        }

        fn add_profile_watcher(&mut self, profile_id: &Uuid, watcher_id: &Uuid) {
            self.real_time_watchers.entry(*profile_id).or_insert(HashSet::new()).insert(*watcher_id);
        }

        fn remove_profile_watcher(&mut self, profile_id: &Uuid, watcher_id: &Uuid) {
            if let Some(watchers) = self.real_time_watchers.get_mut(profile_id) {
                watchers.remove(watcher_id);
            }
        }

        fn get_profile_versions(&self, user_id: &Uuid) -> Vec<ProfileVersion> {
            self.profile_versions.iter()
                .filter(|v| v.user_id == *user_id)
                .cloned()
                .collect()
        }

        fn revert_to_version(&mut self, user_id: &Uuid, version_id: &Uuid) -> Result<(), AppError> {
            let version = self.profile_versions.iter()
                .find(|v| v.version_id == *version_id && v.user_id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "version".to_string(),
                    id: Some(version_id.to_string()),
                })?;

            // Find profile index
            let profile_index = self.profiles.iter()
                .position(|p| p.user_id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "profile".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Revert to version snapshot
            self.profiles[profile_index] = version.profile_snapshot.clone();

            // Log the reversion
            let activity = ActivityLog {
                user_id: *user_id,
                action: "profile_revert".to_string(),
                details: format!("Reverted to version {}", version_id),
                timestamp: Utc::now(),
                ip_address: "127.0.0.1".to_string(),
                user_agent: "TestAgent/1.0".to_string(),
            };
            self.activity_logs.push(activity);

            Ok(())
        }

        fn start_collaboration_session(&mut self, profile_id: &Uuid, participant_ids: Vec<Uuid>) -> Result<Uuid, AppError> {
            let session_id = Uuid::new_v4();
            let mut participants = HashSet::new();
            for id in participant_ids {
                participants.insert(id);
            }

            let session = CollaborationSession {
                session_id,
                profile_id: *profile_id,
                participants,
                active_editors: HashMap::new(),
                pending_changes: HashMap::new(),
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(2),
            };

            self.collaboration_sessions.insert(session_id, session);
            Ok(session_id)
        }

        fn propose_field_change(&mut self, session_id: &Uuid, proposer_id: &Uuid, field_name: &str, new_value: Option<String>) -> Result<(), AppError> {
            let session = self.collaboration_sessions.get_mut(session_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "collaboration_session".to_string(),
                    id: Some(session_id.to_string()),
                })?;

            if !session.participants.contains(proposer_id) {
                return Err(AppError::Authentication {
                    message: "Not a participant in this collaboration session".to_string(),
                });
            }

            let pending_change = PendingChange {
                field_name: field_name.to_string(),
                new_value,
                proposed_by: *proposer_id,
                proposed_at: Utc::now(),
                approvals: HashSet::new(),
                rejections: HashSet::new(),
            };

            session.pending_changes.insert(field_name.to_string(), pending_change);
            Ok(())
        }

        fn approve_field_change(&mut self, session_id: &Uuid, approver_id: &Uuid, field_name: &str) -> Result<bool, AppError> {
            // Extract data to avoid double borrow
            let (profile_id, should_apply, new_value) = {
                let session = self.collaboration_sessions.get_mut(session_id)
                    .ok_or_else(|| AppError::NotFound {
                        resource: "collaboration_session".to_string(),
                        id: Some(session_id.to_string()),
                    })?;

                if !session.participants.contains(approver_id) {
                    return Err(AppError::Authentication {
                        message: "Not a participant in this collaboration session".to_string(),
                    });
                }

                if let Some(pending_change) = session.pending_changes.get_mut(field_name) {
                    pending_change.approvals.insert(*approver_id);

                    // Check if majority approval reached (more than half)
                    let required_approvals = (session.participants.len() / 2) + 1;
                    if pending_change.approvals.len() >= required_approvals {
                        (session.profile_id, true, pending_change.new_value.clone())
                    } else {
                        (session.profile_id, false, None)
                    }
                } else {
                    return Ok(false);
                }
            };

            if should_apply {
                // Apply the change
                self.update_profile_field(&profile_id, field_name, new_value)?;
                // Remove the applied change
                if let Some(session) = self.collaboration_sessions.get_mut(session_id) {
                    session.pending_changes.remove(field_name);
                }
                return Ok(true);
            }

            Ok(false)
        }

        async fn concurrent_profile_updates(&mut self, user_id: &Uuid, updates: Vec<(String, Option<String>)>) -> Vec<Result<(), AppError>> {
            let mut results = Vec::new();

            for (field_name, new_value) in updates {
                let result = self.enhanced_update_profile_field(user_id, &field_name, new_value, "192.168.1.100", "TestAgent/1.0");
                results.push(result);
                sleep(TokioDuration::from_millis(10)).await;
            }

            results
        }

        fn flag_profile_for_moderation(&mut self, user_id: &Uuid, reason: String, flagged_by: &Uuid) {
            let flags = self.moderator_flags.entry(*user_id).or_insert(Vec::new());
            flags.push(format!("{} (flagged by {} at {})", reason, flagged_by, Utc::now()));
        }

        fn get_profile_analytics(&self, user_id: &Uuid) -> Option<&ProfileAnalytics> {
            self.analytics.get(user_id)
        }

        fn track_profile_view(&mut self, profile_id: &Uuid, viewer_id: &Uuid) {
            let analytics = self.analytics.entry(*profile_id).or_insert(ProfileAnalytics {
                user_id: *profile_id,
                view_count: 0,
                unique_viewers: HashSet::new(),
                last_viewed: None,
                update_frequency: HashMap::new(),
                completeness_history: Vec::new(),
            });

            analytics.view_count += 1;
            analytics.unique_viewers.insert(*viewer_id);
            analytics.last_viewed = Some(Utc::now());

            // Create notification if it's a new viewer
            if analytics.unique_viewers.len() == 1 {
                let notification = ProfileNotification {
                    id: Uuid::new_v4(),
                    recipient_id: *profile_id,
                    profile_owner_id: *profile_id,
                    notification_type: NotificationType::ProfileViewed,
                    message: format!("Your profile was viewed by a new user"),
                    read: false,
                    created_at: Utc::now(),
                };
                self.notifications.push(notification);
            }
        }

        fn add_custom_validation_rule(&mut self, field_name: String, rule_type: ValidationType, rule_value: String, error_message: String) {
            let rule = ValidationRule {
                field_name,
                rule_type,
                rule_value,
                error_message,
            };
            self.validation_rules.push(rule);
        }

        fn validate_with_custom_rules(&self, field_name: &str, value: &str) -> Result<(), AppError> {
            for rule in &self.validation_rules {
                if rule.field_name == field_name {
                    match rule.rule_type {
                        ValidationType::MinLength => {
                            if let Ok(min_len) = rule.rule_value.parse::<usize>() {
                                if value.len() < min_len {
                                    return Err(AppError::Validation {
                                        field: field_name.to_string(),
                                        message: rule.error_message.clone(),
                                    });
                                }
                            }
                        }
                        ValidationType::MaxLength => {
                            if let Ok(max_len) = rule.rule_value.parse::<usize>() {
                                if value.len() > max_len {
                                    return Err(AppError::Validation {
                                        field: field_name.to_string(),
                                        message: rule.error_message.clone(),
                                    });
                                }
                            }
                        }
                        ValidationType::Blacklist => {
                            if value.to_lowercase().contains(&rule.rule_value.to_lowercase()) {
                                return Err(AppError::Validation {
                                    field: field_name.to_string(),
                                    message: rule.error_message.clone(),
                                });
                            }
                        }
                        _ => {} // Other types not implemented in this test
                    }
                }
            }
            Ok(())
        }

        fn get_unread_notifications(&self, user_id: &Uuid) -> Vec<&ProfileNotification> {
            self.notifications.iter()
                .filter(|n| n.recipient_id == *user_id && !n.read)
                .collect()
        }

        fn mark_notification_read(&mut self, notification_id: &Uuid) -> Result<(), AppError> {
            let notification = self.notifications.iter_mut()
                .find(|n| n.id == *notification_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "notification".to_string(),
                    id: Some(notification_id.to_string()),
                })?;

            notification.read = true;
            Ok(())
        }

        fn export_profile_data(&self, user_id: &Uuid) -> Result<serde_json::Value, AppError> {
            let profile = self.get_user_profile(user_id)?;
            let history = self.get_profile_update_history(user_id);
            let activity_logs = self.get_activity_logs(user_id);
            let versions = self.get_profile_versions(user_id);
            let analytics = self.get_profile_analytics(user_id);

            Ok(json!({
                "profile": profile,
                "update_history": history,
                "activity_logs": activity_logs,
                "versions": versions,
                "analytics": analytics,
                "exported_at": Utc::now()
            }))
        }
    }

    #[test]
    fn test_create_user_with_empty_profile() {
        let mut simulator = ProfileUpdateSimulator::new();

        let (user, profile) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Verify user was created
        assert_eq!(user.email, "user@example.com");
        assert_eq!(user.username, "testuser");

        // Verify empty profile was created
        assert_eq!(profile.user_id, user.id);
        assert!(profile.first_name.is_none());
        assert!(profile.last_name.is_none());
        assert!(profile.bio.is_none());
        assert!(profile.location.is_none());
        assert!(profile.website.is_none());
        assert!(profile.avatar_url.is_none());
        assert!(profile.preferences_json.is_none());
    }

    #[test]
    fn test_update_single_profile_field() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Update first name
        let result = simulator.update_profile_field(
            &user.id,
            "first_name",
            Some("John".to_string())
        );
        assert!(result.is_ok());

        // Verify update
        let updated_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(updated_profile.first_name, Some("John".to_string()));

        // Verify update event was recorded
        let history = simulator.get_profile_update_history(&user.id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].field_name, "first_name");
        assert_eq!(history[0].old_value, None);
        assert_eq!(history[0].new_value, Some("John".to_string()));
    }

    #[test]
    fn test_bulk_profile_update() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let updates = vec![
            ("first_name".to_string(), Some("John".to_string())),
            ("last_name".to_string(), Some("Doe".to_string())),
            ("bio".to_string(), Some("Software developer".to_string())),
            ("location".to_string(), Some("San Francisco, CA".to_string())),
        ];

        let result = simulator.bulk_update_profile(&user.id, updates);
        assert!(result.is_ok());

        let updated_profile = result.unwrap();
        assert_eq!(updated_profile.first_name, Some("John".to_string()));
        assert_eq!(updated_profile.last_name, Some("Doe".to_string()));
        assert_eq!(updated_profile.bio, Some("Software developer".to_string()));
        assert_eq!(updated_profile.location, Some("San Francisco, CA".to_string()));

        // Verify all update events were recorded
        let history = simulator.get_profile_update_history(&user.id);
        assert_eq!(history.len(), 4);
    }

    #[test]
    fn test_profile_field_validation() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Test name length validation
        let long_name = "a".repeat(51);
        let result = simulator.update_profile_field(&user.id, "first_name", Some(long_name));
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "first_name");
                assert!(message.contains("50 characters"));
            }
            _ => panic!("Expected validation error"),
        }

        // Test bio length validation
        let long_bio = "a".repeat(501);
        let result = simulator.update_profile_field(&user.id, "bio", Some(long_bio));
        assert!(result.is_err());

        // Test website URL validation
        let invalid_url = "not-a-url";
        let result = simulator.update_profile_field(&user.id, "website", Some(invalid_url.to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "website");
                assert!(message.contains("valid URL"));
            }
            _ => panic!("Expected validation error"),
        }

        // Test valid website URL
        let valid_url = "https://example.com";
        let result = simulator.update_profile_field(&user.id, "website", Some(valid_url.to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_preferences_json_validation() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Test invalid JSON
        let invalid_json = "{invalid json";
        let result = simulator.update_profile_field(&user.id, "preferences_json", Some(invalid_json.to_string()));
        assert!(result.is_err());

        // Test valid JSON
        let valid_json = r#"{"theme": "dark", "notifications": true, "language": "en"}"#;
        let result = simulator.update_profile_field(&user.id, "preferences_json", Some(valid_json.to_string()));
        assert!(result.is_ok());

        // Verify JSON was stored
        let profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(profile.preferences_json, Some(valid_json.to_string()));

        // Test JSON size limit
        let large_json = format!(r#"{{"data": "{}"}}"#, "a".repeat(5000));
        let result = simulator.update_profile_field(&user.id, "preferences_json", Some(large_json));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_profile_field() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Add a field first
        simulator.update_profile_field(&user.id, "bio", Some("Original bio".to_string())).unwrap();

        // Verify field exists
        let profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(profile.bio, Some("Original bio".to_string()));

        // Delete the field
        let result = simulator.delete_profile_field(&user.id, "bio");
        assert!(result.is_ok());

        // Verify field was deleted
        let updated_profile = simulator.get_user_profile(&user.id).unwrap();
        assert!(updated_profile.bio.is_none());

        // Verify deletion was logged
        let history = simulator.get_profile_update_history(&user.id);
        let deletion_event = history.iter().find(|e| e.new_value.is_none()).unwrap();
        assert_eq!(deletion_event.field_name, "bio");
        assert_eq!(deletion_event.old_value, Some("Original bio".to_string()));
    }

    #[test]
    fn test_profile_update_for_inactive_user() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (mut user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Make user inactive
        user.status = UserStatus::Inactive;
        simulator.users[0] = user.clone();

        // Try to update profile
        let result = simulator.update_profile_field(&user.id, "first_name", Some("John".to_string()));
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "user");
                assert!(message.contains("inactive"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_profile_completeness_calculation() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Empty profile should be 0% complete
        let completeness = simulator.get_profile_completeness(&user.id).unwrap();
        assert_eq!(completeness, 0.0);

        // Add some fields
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "last_name", Some("Doe".to_string())).unwrap();

        // Should be 2/7 = ~28.57% complete
        let completeness = simulator.get_profile_completeness(&user.id).unwrap();
        assert!((completeness - 28.571428).abs() < 0.001);

        // Add all remaining fields
        simulator.update_profile_field(&user.id, "bio", Some("Bio".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "location", Some("Location".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "website", Some("https://example.com".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "avatar_url", Some("https://example.com/avatar.jpg".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "preferences_json", Some(r#"{"theme":"dark"}"#.to_string())).unwrap();

        // Should be 100% complete
        let completeness = simulator.get_profile_completeness(&user.id).unwrap();
        assert_eq!(completeness, 100.0);
    }

    #[test]
    fn test_profile_search_functionality() {
        let mut simulator = ProfileUpdateSimulator::new();

        // Create multiple users with profiles
        let (user1, _) = simulator.create_test_user_with_profile("user1@example.com".to_string(), "user1".to_string());
        let (user2, _) = simulator.create_test_user_with_profile("user2@example.com".to_string(), "user2".to_string());
        let (user3, _) = simulator.create_test_user_with_profile("user3@example.com".to_string(), "user3".to_string());

        // Update profiles with different information
        simulator.update_profile_field(&user1.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user1.id, "location", Some("San Francisco".to_string())).unwrap();

        simulator.update_profile_field(&user2.id, "first_name", Some("Jane".to_string())).unwrap();
        simulator.update_profile_field(&user2.id, "location", Some("New York".to_string())).unwrap();

        simulator.update_profile_field(&user3.id, "first_name", Some("Bob".to_string())).unwrap();
        simulator.update_profile_field(&user3.id, "location", Some("San Diego".to_string())).unwrap();

        // Search by first name
        let john_results = simulator.search_profiles_by_field("first_name", "John");
        assert_eq!(john_results.len(), 1);
        assert_eq!(john_results[0].user_id, user1.id);

        // Search by location (partial match)
        let san_results = simulator.search_profiles_by_field("location", "San");
        assert_eq!(san_results.len(), 2); // San Francisco and San Diego

        // Search with no results
        let no_results = simulator.search_profiles_by_field("first_name", "NonExistent");
        assert_eq!(no_results.len(), 0);
    }

    #[test]
    fn test_profile_update_activity_logging() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Perform multiple updates
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Software developer".to_string())).unwrap();
        simulator.delete_profile_field(&user.id, "bio").unwrap();

        // Check activity logs
        let activity_logs = simulator.get_activity_logs(&user.id);
        assert_eq!(activity_logs.len(), 3);

        // Verify first update
        assert_eq!(activity_logs[0].action, "profile_update");
        assert!(activity_logs[0].details.contains("first_name"));
        assert!(activity_logs[0].details.contains("John"));

        // Verify bio addition
        assert!(activity_logs[1].details.contains("bio"));
        assert!(activity_logs[1].details.contains("Software developer"));

        // Verify bio deletion
        assert!(activity_logs[2].details.contains("bio"));
        assert!(activity_logs[2].details.contains("None"));
    }

    #[test]
    fn test_profile_privacy_validation() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user1, _) = simulator.create_test_user_with_profile(
            "user1@example.com".to_string(),
            "user1".to_string()
        );
        let (user2, _) = simulator.create_test_user_with_profile(
            "user2@example.com".to_string(),
            "user2".to_string()
        );

        // User can access their own profile
        let self_access = simulator.validate_profile_privacy(&user1.id, &user1.id);
        assert!(self_access.is_ok());
        assert!(self_access.unwrap());

        // User can access other profiles (public by default)
        let other_access = simulator.validate_profile_privacy(&user1.id, &user2.id);
        assert!(other_access.is_ok());
        assert!(other_access.unwrap());
    }

    #[test]
    fn test_profile_update_history_tracking() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Perform multiple updates to same field
        simulator.update_profile_field(&user.id, "bio", Some("First bio".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Second bio".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Third bio".to_string())).unwrap();

        let history = simulator.get_profile_update_history(&user.id);
        assert_eq!(history.len(), 3);

        // Verify chronological order and value changes
        assert_eq!(history[0].old_value, None);
        assert_eq!(history[0].new_value, Some("First bio".to_string()));

        assert_eq!(history[1].old_value, Some("First bio".to_string()));
        assert_eq!(history[1].new_value, Some("Second bio".to_string()));

        assert_eq!(history[2].old_value, Some("Second bio".to_string()));
        assert_eq!(history[2].new_value, Some("Third bio".to_string()));
    }

    #[test]
    fn test_concurrent_profile_updates() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Simulate concurrent updates (in real system, would need proper locking)
        let updates1 = vec![
            ("first_name".to_string(), Some("John".to_string())),
            ("last_name".to_string(), Some("Doe".to_string())),
        ];

        let updates2 = vec![
            ("bio".to_string(), Some("Software developer".to_string())),
            ("location".to_string(), Some("San Francisco".to_string())),
        ];

        // Apply both sets of updates
        simulator.bulk_update_profile(&user.id, updates1).unwrap();
        simulator.bulk_update_profile(&user.id, updates2).unwrap();

        // Verify all updates were applied
        let final_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(final_profile.first_name, Some("John".to_string()));
        assert_eq!(final_profile.last_name, Some("Doe".to_string()));
        assert_eq!(final_profile.bio, Some("Software developer".to_string()));
        assert_eq!(final_profile.location, Some("San Francisco".to_string()));

        // Verify all events were recorded
        let history = simulator.get_profile_update_history(&user.id);
        assert_eq!(history.len(), 4);
    }

    #[test]
    fn test_profile_update_with_special_characters() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Test with various special characters and unicode
        let special_bio = "🚀 Full-stack developer with expertise in Rust & TypeScript. 日本語も話せます! 🎯";
        let result = simulator.update_profile_field(&user.id, "bio", Some(special_bio.to_string()));
        assert!(result.is_ok());

        let updated_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(updated_profile.bio, Some(special_bio.to_string()));

        // Test with JSON containing special characters
        let special_json = r#"{"theme": "dark", "motto": "Code & ☕", "languages": ["English", "日本語"]}"#;
        let result = simulator.update_profile_field(&user.id, "preferences_json", Some(special_json.to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_field_edge_cases() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Test empty string (should be treated as None)
        let result = simulator.update_profile_field(&user.id, "first_name", Some("".to_string()));
        assert!(result.is_ok());

        // Test updating non-existent field
        let result = simulator.update_profile_field(&user.id, "invalid_field", Some("value".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "field_name");
                assert!(message.contains("Unknown profile field"));
            }
            _ => panic!("Expected validation error"),
        }

        // Test minimum valid URLs
        let min_http_url = "http://a.b";
        let result = simulator.update_profile_field(&user.id, "website", Some(min_http_url.to_string()));
        assert!(result.is_ok());

        let min_https_url = "https://x.y";
        let result = simulator.update_profile_field(&user.id, "avatar_url", Some(min_https_url.to_string()));
        assert!(result.is_ok());
    }

    // Enhanced Integration Tests with Advanced Features

    #[test]
    fn test_enhanced_profile_update_with_rate_limiting() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let ip = "192.168.1.100";
        let user_agent = "Mozilla/5.0";

        // Perform multiple updates quickly
        for i in 0..3 {
            let result = simulator.enhanced_update_profile_field(
                &user.id,
                "bio",
                Some(format!("Update {}", i)),
                ip,
                user_agent
            );
            assert!(result.is_ok(), "Update {} failed", i);
        }

        // 4th update should trigger rate limit
        let result = simulator.enhanced_update_profile_field(
            &user.id,
            "bio",
            Some("Should be blocked".to_string()),
            ip,
            user_agent
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::TooManyRequests { .. } => {},
            _ => panic!("Expected rate limiting error"),
        }

        // Different IP should work
        let result = simulator.enhanced_update_profile_field(
            &user.id,
            "bio",
            Some("Different IP works".to_string()),
            "192.168.1.101",
            user_agent
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_moderation() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Test blocked words
        let result = simulator.enhanced_update_profile_field(
            &user.id,
            "bio",
            Some("This contains spam content".to_string()),
            "192.168.1.100",
            "TestAgent"
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { message, .. } => {
                assert!(message.contains("contains blocked content"));
            },
            _ => panic!("Expected content moderation error"),
        }

        // Test excessive caps
        let result = simulator.enhanced_update_profile_field(
            &user.id,
            "bio",
            Some("THIS IS ALL CAPS AND SHOULD BE BLOCKED".to_string()),
            "192.168.1.100",
            "TestAgent"
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { message, .. } => {
                assert!(message.contains("too many capital letters"));
            },
            _ => panic!("Expected content moderation error"),
        }

        // Test allowed content
        let result = simulator.enhanced_update_profile_field(
            &user.id,
            "bio",
            Some("This is a normal bio with proper capitalization.".to_string()),
            "192.168.1.100",
            "TestAgent"
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_versioning_and_rollback() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Initial state
        let initial_profile = simulator.get_user_profile(&user.id).unwrap();

        // Make some updates
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "last_name", Some("Doe".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Software engineer".to_string())).unwrap();

        let updated_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(updated_profile.first_name, Some("John".to_string()));
        assert_eq!(updated_profile.last_name, Some("Doe".to_string()));
        assert_eq!(updated_profile.bio, Some("Software engineer".to_string()));

        // Check that versions were created
        let versions = simulator.get_profile_versions(&user.id);
        assert!(!versions.is_empty());

        // Revert to a previous version
        if let Some(version) = versions.first() {
            let result = simulator.revert_to_version(&user.id, &version.version_id);
            assert!(result.is_ok());

            // Verify rollback worked
            let reverted_profile = simulator.get_user_profile(&user.id).unwrap();
            assert_eq!(reverted_profile.first_name, initial_profile.first_name);
            assert_eq!(reverted_profile.last_name, initial_profile.last_name);
            assert_eq!(reverted_profile.bio, initial_profile.bio);
        }
    }

    #[test]
    fn test_profile_caching() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Get profile first, then cache it
        let profile = simulator.get_user_profile(&user.id).unwrap();
        simulator.cache_profile(&user.id, &profile, 60); // 60 second TTL

        // Get cached profile
        let cached_profile = simulator.get_cached_profile(&user.id);
        assert!(cached_profile.is_some());

        // Update profile
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();

        // Cached profile should still be old version
        let still_cached = simulator.get_cached_profile(&user.id);
        assert!(still_cached.is_some());
        assert_ne!(still_cached.unwrap().first_name, Some("John".to_string()));

        // Invalidate cache
        simulator.invalidate_cache(&user.id);

        // Should no longer be cached
        let no_cache = simulator.get_cached_profile(&user.id);
        assert!(no_cache.is_none());
    }

    #[test]
    fn test_collaboration_sessions() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let collaborator1_id = Uuid::new_v4();
        let collaborator2_id = Uuid::new_v4();

        // Start collaboration session
        let session_id = simulator.start_collaboration_session(
            &user.id,
            vec![collaborator1_id, collaborator2_id]
        ).unwrap();

        // Propose changes
        let changes = vec![
            ("first_name".to_string(), Some("Team".to_string())),
            ("last_name".to_string(), Some("Lead".to_string()))
        ];

        let result = simulator.propose_collaboration_changes(&session_id, &collaborator1_id, changes);
        assert!(result.is_ok());

        // Approve changes
        let result = simulator.approve_collaboration_changes(&session_id, &collaborator2_id);
        assert!(result.is_ok());

        // Apply approved changes
        let result = simulator.apply_collaboration_changes(&session_id);
        assert!(result.is_ok());

        // Verify changes were applied
        let updated_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(updated_profile.first_name, Some("Team".to_string()));
        assert_eq!(updated_profile.last_name, Some("Lead".to_string()));

        // End session
        let result = simulator.end_collaboration_session(&session_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_real_time_notifications() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        let watcher_id = Uuid::new_v4();

        // Add profile watcher
        let result = simulator.add_profile_watcher(&user.id, &watcher_id);
        assert!(result.is_ok());

        // Update profile
        simulator.update_profile_field(&user.id, "bio", Some("New bio".to_string())).unwrap();

        // Check notifications were sent
        let notifications = simulator.get_notifications(&watcher_id);
        assert!(!notifications.is_empty());

        let notification = &notifications[0];
        assert_eq!(notification.user_id, user.id);
        assert!(notification.message.contains("bio"));

        // Remove watcher
        let result = simulator.remove_profile_watcher(&user.id, &watcher_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_analytics() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Perform various updates
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "last_name", Some("Doe".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Engineer".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "location", Some("NYC".to_string())).unwrap();

        // Get analytics
        let analytics = simulator.get_profile_analytics(&user.id).unwrap();
        assert_eq!(analytics.total_updates, 4);
        assert_eq!(analytics.fields_updated, 4);
        assert!(analytics.last_updated.is_some());
        assert!(analytics.avg_updates_per_day >= 0.0);
    }

    #[test]
    fn test_custom_validation_rules() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Add custom validation rule
        let rule_name = "bio_professional".to_string();
        let validation_fn = "function(value) { return value.includes('professional') || value.includes('engineer') || value.includes('developer'); }".to_string();
        let error_message = "Bio must contain professional keywords".to_string();

        let result = simulator.add_custom_validation_rule(
            "bio".to_string(),
            rule_name.clone(),
            validation_fn,
            error_message.clone()
        );
        assert!(result.is_ok());

        // Test validation passes
        let result = simulator.update_profile_field(&user.id, "bio", Some("I am a professional engineer".to_string()));
        assert!(result.is_ok());

        // Test validation fails
        let result = simulator.update_profile_field(&user.id, "bio", Some("I like music and art".to_string()));
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { message, .. } => {
                assert_eq!(message, error_message);
            },
            _ => panic!("Expected custom validation error"),
        }

        // Remove validation rule
        let result = simulator.remove_custom_validation_rule("bio".to_string(), rule_name);
        assert!(result.is_ok());

        // Now the update should work
        let result = simulator.update_profile_field(&user.id, "bio", Some("I like music and art".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_profile_export() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Update profile with some data
        simulator.update_profile_field(&user.id, "first_name", Some("John".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "last_name", Some("Doe".to_string())).unwrap();
        simulator.update_profile_field(&user.id, "bio", Some("Software engineer".to_string())).unwrap();

        // Export profile data
        let export_data = simulator.export_profile_data(&user.id).unwrap();

        // Verify export contains expected data
        assert!(export_data.contains("first_name"));
        assert!(export_data.contains("John"));
        assert!(export_data.contains("last_name"));
        assert!(export_data.contains("Doe"));
        assert!(export_data.contains("bio"));
        assert!(export_data.contains("Software engineer"));

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&export_data).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_enhanced_concurrent_profile_updates() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Simulate concurrent updates by rapidly updating different fields
        let updates = vec![
            ("first_name", Some("John".to_string())),
            ("last_name", Some("Doe".to_string())),
            ("bio", Some("Engineer".to_string())),
            ("location", Some("NYC".to_string())),
            ("website", Some("https://johndoe.dev".to_string())),
        ];

        // Apply all updates
        for (field, value) in updates {
            let result = simulator.update_profile_field(&user.id, field, value);
            assert!(result.is_ok(), "Failed to update {}", field);
        }

        // Verify all updates were applied
        let final_profile = simulator.get_user_profile(&user.id).unwrap();
        assert_eq!(final_profile.first_name, Some("John".to_string()));
        assert_eq!(final_profile.last_name, Some("Doe".to_string()));
        assert_eq!(final_profile.bio, Some("Engineer".to_string()));
        assert_eq!(final_profile.location, Some("NYC".to_string()));
        assert_eq!(final_profile.website, Some("https://johndoe.dev".to_string()));

        // Check that all update events were recorded
        let history = simulator.get_profile_update_history(&user.id);
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_profile_security_audit() {
        let mut simulator = ProfileUpdateSimulator::new();
        let (user, _) = simulator.create_test_user_with_profile(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Perform updates from different IPs to trigger security checks
        let ips = vec!["192.168.1.100", "10.0.0.50", "172.16.0.25"];

        for (i, ip) in ips.iter().enumerate() {
            let result = simulator.enhanced_update_profile_field(
                &user.id,
                "bio",
                Some(format!("Update from IP {}", i)),
                ip,
                "TestAgent"
            );
            assert!(result.is_ok());
        }

        // Check activity logs for security patterns
        let activities = simulator.get_activity_logs(&user.id);
        let unique_ips: std::collections::HashSet<_> = activities.iter()
            .map(|log| &log.ip_address)
            .collect();

        // Should have detected multiple IPs
        assert!(unique_ips.len() >= 3);

        // Check for any security events (in a real system)
        // This would check for suspicious patterns, geo-location changes, etc.
        let analytics = simulator.get_profile_analytics(&user.id).unwrap();
        assert!(analytics.total_updates >= 3);
    }
}