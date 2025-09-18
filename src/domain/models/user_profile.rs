use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::domain::Model;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
    pub preferences: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Model for UserProfile {}

impl UserProfile {
    pub fn new(user_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            first_name: None,
            last_name: None,
            avatar_url: None,
            bio: None,
            website: None,
            location: None,
            preferences: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn full_name(&self) -> Option<String> {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            (Some(first), None) => Some(first.clone()),
            (None, Some(last)) => Some(last.clone()),
            (None, None) => None,
        }
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now();
    }

    pub fn update_name(&mut self, first_name: Option<String>, last_name: Option<String>) {
        self.first_name = first_name;
        self.last_name = last_name;
        self.update_timestamp();
    }

    pub fn update_bio(&mut self, bio: Option<String>) {
        self.bio = bio;
        self.update_timestamp();
    }

    pub fn update_avatar(&mut self, avatar_url: Option<String>) {
        self.avatar_url = avatar_url;
        self.update_timestamp();
    }

    pub fn update_website(&mut self, website: Option<String>) {
        self.website = website;
        self.update_timestamp();
    }

    pub fn update_location(&mut self, location: Option<String>) {
        self.location = location;
        self.update_timestamp();
    }

    pub fn update_preference(&mut self, key: &str, value: serde_json::Value) {
        if let Some(preferences) = self.preferences.as_object_mut() {
            preferences.insert(key.to_string(), value);
        }
        self.update_timestamp();
    }

    pub fn get_preference(&self, key: &str) -> Option<&serde_json::Value> {
        self.preferences.as_object()?.get(key)
    }

    pub fn remove_preference(&mut self, key: &str) {
        if let Some(preferences) = self.preferences.as_object_mut() {
            preferences.remove(key);
        }
        self.update_timestamp();
    }

    pub fn has_complete_name(&self) -> bool {
        self.first_name.is_some() && self.last_name.is_some()
    }

    pub fn profile_completeness_score(&self) -> f32 {
        let mut score = 0.0;
        let total_fields = 6.0;

        if self.first_name.is_some() { score += 1.0; }
        if self.last_name.is_some() { score += 1.0; }
        if self.bio.is_some() { score += 1.0; }
        if self.website.is_some() { score += 1.0; }
        if self.location.is_some() { score += 1.0; }
        if self.avatar_url.is_some() { score += 1.0; }

        (score / total_fields) * 100.0
    }

    pub fn is_url_valid(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    pub fn validate_website_url(&self) -> bool {
        match &self.website {
            Some(url) => Self::is_url_valid(url),
            None => true, // No URL is valid
        }
    }

    pub fn validate_avatar_url(&self) -> bool {
        match &self.avatar_url {
            Some(url) => Self::is_url_valid(url),
            None => true, // No URL is valid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_profile_creation() {
        let user_id = Uuid::now_v7();
        let profile = UserProfile::new(user_id);

        assert_eq!(profile.user_id, user_id);
        assert!(profile.first_name.is_none());
        assert!(profile.last_name.is_none());
        assert!(profile.avatar_url.is_none());
        assert!(profile.bio.is_none());
        assert!(profile.website.is_none());
        assert!(profile.location.is_none());
        assert_eq!(profile.preferences, json!({}));

        // Timestamps should be recent and equal
        let now = Utc::now();
        assert!((now - profile.created_at).num_seconds() < 1);
        assert!((now - profile.updated_at).num_seconds() < 1);
        assert_eq!(profile.created_at, profile.updated_at);
    }

    #[test]
    fn test_full_name_combinations() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // No names
        assert!(profile.full_name().is_none());

        // First name only
        profile.first_name = Some("John".to_string());
        assert_eq!(profile.full_name(), Some("John".to_string()));

        // Last name only
        profile.first_name = None;
        profile.last_name = Some("Doe".to_string());
        assert_eq!(profile.full_name(), Some("Doe".to_string()));

        // Both names
        profile.first_name = Some("John".to_string());
        assert_eq!(profile.full_name(), Some("John Doe".to_string()));

        // Empty strings should still work
        profile.first_name = Some(String::new());
        profile.last_name = Some(String::new());
        assert_eq!(profile.full_name(), Some(" ".to_string()));
    }

    #[test]
    fn test_update_name() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);
        let initial_updated_at = profile.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));

        profile.update_name(
            Some("Jane".to_string()),
            Some("Smith".to_string()),
        );

        assert_eq!(profile.first_name, Some("Jane".to_string()));
        assert_eq!(profile.last_name, Some("Smith".to_string()));
        assert_eq!(profile.full_name(), Some("Jane Smith".to_string()));
        assert!(profile.updated_at > initial_updated_at);
    }

    #[test]
    fn test_update_bio() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);
        let initial_updated_at = profile.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));

        let bio = "Software developer passionate about Rust".to_string();
        profile.update_bio(Some(bio.clone()));

        assert_eq!(profile.bio, Some(bio));
        assert!(profile.updated_at > initial_updated_at);

        // Test clearing bio
        let second_updated_at = profile.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(1));
        profile.update_bio(None);

        assert!(profile.bio.is_none());
        assert!(profile.updated_at > second_updated_at);
    }

    #[test]
    fn test_update_avatar() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        let avatar_url = "https://example.com/avatar.jpg".to_string();
        profile.update_avatar(Some(avatar_url.clone()));

        assert_eq!(profile.avatar_url, Some(avatar_url));
    }

    #[test]
    fn test_update_website() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        let website = "https://johndoe.com".to_string();
        profile.update_website(Some(website.clone()));

        assert_eq!(profile.website, Some(website));
    }

    #[test]
    fn test_update_location() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        let location = "San Francisco, CA".to_string();
        profile.update_location(Some(location.clone()));

        assert_eq!(profile.location, Some(location));
    }

    #[test]
    fn test_preferences_management() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // Add preferences
        profile.update_preference("theme", json!("dark"));
        profile.update_preference("notifications", json!(true));
        profile.update_preference("language", json!("en"));

        // Check preferences were added
        assert_eq!(profile.get_preference("theme"), Some(&json!("dark")));
        assert_eq!(profile.get_preference("notifications"), Some(&json!(true)));
        assert_eq!(profile.get_preference("language"), Some(&json!("en")));

        // Check non-existent preference
        assert!(profile.get_preference("non_existent").is_none());

        // Remove preference
        profile.remove_preference("notifications");
        assert!(profile.get_preference("notifications").is_none());

        // Other preferences should still exist
        assert_eq!(profile.get_preference("theme"), Some(&json!("dark")));
        assert_eq!(profile.get_preference("language"), Some(&json!("en")));
    }

    #[test]
    fn test_has_complete_name() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // No names
        assert!(!profile.has_complete_name());

        // First name only
        profile.first_name = Some("John".to_string());
        assert!(!profile.has_complete_name());

        // Last name only
        profile.first_name = None;
        profile.last_name = Some("Doe".to_string());
        assert!(!profile.has_complete_name());

        // Both names
        profile.first_name = Some("John".to_string());
        assert!(profile.has_complete_name());

        // Empty strings count as "some"
        profile.first_name = Some(String::new());
        profile.last_name = Some(String::new());
        assert!(profile.has_complete_name());
    }

    #[test]
    fn test_profile_completeness_score() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // Empty profile
        assert_eq!(profile.profile_completeness_score(), 0.0);

        // Add each field and check score
        profile.first_name = Some("John".to_string());
        assert!((profile.profile_completeness_score() - 16.67).abs() < 0.01);

        profile.last_name = Some("Doe".to_string());
        assert!((profile.profile_completeness_score() - 33.33).abs() < 0.01);

        profile.bio = Some("Software developer".to_string());
        assert_eq!(profile.profile_completeness_score(), 50.0);

        profile.website = Some("https://johndoe.com".to_string());
        assert!((profile.profile_completeness_score() - 66.67).abs() < 0.01);

        profile.location = Some("San Francisco".to_string());
        assert!((profile.profile_completeness_score() - 83.33).abs() < 0.01);

        profile.avatar_url = Some("https://example.com/avatar.jpg".to_string());
        assert_eq!(profile.profile_completeness_score(), 100.0);
    }

    #[test]
    fn test_url_validation() {
        // Valid URLs
        assert!(UserProfile::is_url_valid("http://example.com"));
        assert!(UserProfile::is_url_valid("https://example.com"));
        assert!(UserProfile::is_url_valid("https://subdomain.example.com/path"));
        assert!(UserProfile::is_url_valid("http://localhost:8080"));

        // Invalid URLs
        assert!(!UserProfile::is_url_valid("ftp://example.com"));
        assert!(!UserProfile::is_url_valid("example.com"));
        assert!(!UserProfile::is_url_valid("www.example.com"));
        assert!(!UserProfile::is_url_valid(""));
        assert!(!UserProfile::is_url_valid("not a url"));
    }

    #[test]
    fn test_website_url_validation() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // No website is valid
        assert!(profile.validate_website_url());

        // Valid website
        profile.website = Some("https://johndoe.com".to_string());
        assert!(profile.validate_website_url());

        // Invalid website
        profile.website = Some("johndoe.com".to_string());
        assert!(!profile.validate_website_url());
    }

    #[test]
    fn test_avatar_url_validation() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // No avatar is valid
        assert!(profile.validate_avatar_url());

        // Valid avatar URL
        profile.avatar_url = Some("https://example.com/avatar.jpg".to_string());
        assert!(profile.validate_avatar_url());

        // Invalid avatar URL
        profile.avatar_url = Some("example.com/avatar.jpg".to_string());
        assert!(!profile.validate_avatar_url());
    }

    #[test]
    fn test_update_timestamp() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);
        let initial_updated_at = profile.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        profile.update_timestamp();

        assert!(profile.updated_at > initial_updated_at);
        // Created timestamp should remain unchanged
        assert_eq!(profile.created_at, profile.created_at);
    }

    #[test]
    fn test_profile_serialization() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        profile.first_name = Some("John".to_string());
        profile.last_name = Some("Doe".to_string());
        profile.bio = Some("Software developer".to_string());
        profile.update_preference("theme", json!("dark"));

        let serialized = serde_json::to_string(&profile).unwrap();
        let deserialized: UserProfile = serde_json::from_str(&serialized).unwrap();

        assert_eq!(profile.user_id, deserialized.user_id);
        assert_eq!(profile.first_name, deserialized.first_name);
        assert_eq!(profile.last_name, deserialized.last_name);
        assert_eq!(profile.bio, deserialized.bio);
        assert_eq!(profile.preferences, deserialized.preferences);
    }

    #[test]
    fn test_profile_clone() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        profile.first_name = Some("John".to_string());
        profile.last_name = Some("Doe".to_string());
        profile.update_preference("theme", json!("dark"));

        let cloned_profile = profile.clone();

        assert_eq!(profile.user_id, cloned_profile.user_id);
        assert_eq!(profile.first_name, cloned_profile.first_name);
        assert_eq!(profile.last_name, cloned_profile.last_name);
        assert_eq!(profile.preferences, cloned_profile.preferences);
        assert_eq!(profile.created_at, cloned_profile.created_at);
        assert_eq!(profile.updated_at, cloned_profile.updated_at);
    }

    #[test]
    fn test_complex_preferences() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // Add complex nested preferences
        profile.update_preference("ui", json!({
            "theme": "dark",
            "sidebar_collapsed": true,
            "font_size": 14
        }));

        profile.update_preference("notifications", json!({
            "email": true,
            "push": false,
            "sms": false,
            "frequency": "daily"
        }));

        // Verify complex preferences
        let ui_pref = profile.get_preference("ui").unwrap();
        assert_eq!(ui_pref["theme"], json!("dark"));
        assert_eq!(ui_pref["sidebar_collapsed"], json!(true));
        assert_eq!(ui_pref["font_size"], json!(14));

        let notif_pref = profile.get_preference("notifications").unwrap();
        assert_eq!(notif_pref["email"], json!(true));
        assert_eq!(notif_pref["frequency"], json!("daily"));
    }

    #[test]
    fn test_edge_cases() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // Test with empty strings
        profile.update_name(Some(String::new()), Some(String::new()));
        assert_eq!(profile.full_name(), Some(" ".to_string()));

        // Test with very long strings
        let long_string = "a".repeat(1000);
        profile.update_bio(Some(long_string.clone()));
        assert_eq!(profile.bio.as_ref().unwrap().len(), 1000);

        // Test with special characters
        profile.update_name(
            Some("José".to_string()),
            Some("García-Rodríguez".to_string()),
        );
        assert_eq!(profile.full_name(), Some("José García-Rodríguez".to_string()));

        // Test with emoji
        profile.update_bio(Some("🚀 Rust developer 🦀".to_string()));
        assert!(profile.bio.unwrap().contains("🚀"));
    }

    #[test]
    fn test_profile_lifecycle() {
        let user_id = Uuid::now_v7();
        let mut profile = UserProfile::new(user_id);

        // Start with empty profile
        assert_eq!(profile.profile_completeness_score(), 0.0);
        assert!(!profile.has_complete_name());
        assert!(profile.full_name().is_none());

        // Gradually complete profile
        profile.update_name(Some("John".to_string()), Some("Doe".to_string()));
        assert!(profile.has_complete_name());
        assert_eq!(profile.full_name(), Some("John Doe".to_string()));

        profile.update_bio(Some("Software engineer with 5 years of experience".to_string()));
        profile.update_location(Some("San Francisco, CA".to_string()));
        profile.update_website(Some("https://johndoe.dev".to_string()));
        profile.update_avatar(Some("https://example.com/john.jpg".to_string()));

        // Should be complete now
        assert_eq!(profile.profile_completeness_score(), 100.0);
        assert!(profile.validate_website_url());
        assert!(profile.validate_avatar_url());

        // Add preferences
        profile.update_preference("theme", json!("dark"));
        profile.update_preference("language", json!("en"));
        profile.update_preference("timezone", json!("America/Los_Angeles"));

        // Verify all data is intact
        assert_eq!(profile.full_name(), Some("John Doe".to_string()));
        assert!(profile.bio.is_some());
        assert!(profile.location.is_some());
        assert!(profile.website.is_some());
        assert!(profile.avatar_url.is_some());
        assert_eq!(profile.get_preference("theme"), Some(&json!("dark")));

        // Clear some data
        profile.update_bio(None);
        assert!(profile.bio.is_none());
        assert!(profile.profile_completeness_score() < 100.0);
    }
}