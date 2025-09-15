//! Tests for UserProfile entity
//!
//! This module contains comprehensive tests for the UserProfile model,
//! including validation, serialization, and business logic.

#[cfg(test)]
mod user_profile_tests {
    use chrono::Utc;
    use uuid::Uuid;
    use serde_json;

    use crate::models::UserProfile;

    // Helper function to create a test profile
    fn create_test_profile() -> UserProfile {
        UserProfile {
            user_id: Uuid::new_v4(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            bio: Some("Software engineer with 10 years of experience".to_string()),
            location: Some("San Francisco, CA".to_string()),
            website: Some("https://johndoe.dev".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            preferences: serde_json::json!({"theme": "dark", "notifications": true}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_profile_creation() {
        let profile = create_test_profile();

        assert!(!profile.user_id.to_string().is_empty());
        assert_eq!(profile.first_name, Some("John".to_string()));
        assert_eq!(profile.last_name, Some("Doe".to_string()));
        assert!(profile.bio.is_some());
        assert!(profile.location.is_some());
        assert!(profile.website.is_some());
    }

    #[test]
    fn test_profile_with_minimal_data() {
        let profile = UserProfile {
            user_id: Uuid::new_v4(),
            first_name: None,
            last_name: None,
            bio: None,
            location: None,
            website: None,
            avatar_url: None,
            preferences: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(profile.first_name.is_none());
        assert!(profile.last_name.is_none());
        assert!(profile.bio.is_none());
        assert!(profile.location.is_none());
    }

    #[test]
    fn test_profile_serialization() {
        let profile = create_test_profile();

        // Serialize to JSON
        let json = serde_json::to_string(&profile).expect("Failed to serialize profile");
        assert!(json.contains("\"first_name\":\"John\""));
        assert!(json.contains("\"last_name\":\"Doe\""));

        // Deserialize from JSON
        let deserialized: UserProfile = serde_json::from_str(&json)
            .expect("Failed to deserialize profile");
        assert_eq!(deserialized.first_name, profile.first_name);
        assert_eq!(deserialized.last_name, profile.last_name);
        assert_eq!(deserialized.user_id, profile.user_id);
    }

    #[test]
    fn test_profile_name_validation() {
        // Valid names
        let valid_names = vec![
            "John",
            "Mary-Jane",
            "José",
            "François",
            "李明", // Chinese characters
            "O'Brien",
            "Van Der Berg",
        ];

        for name in valid_names {
            assert!(validate_name(name), "Name '{}' should be valid", name);
        }

        // Invalid names
        let long_name = "a".repeat(51); // Too long (>50 chars)
        let invalid_names = vec![
            "", // Empty
            "J", // Too short (assuming min 2 chars)
            "123", // Numbers only
            "John123", // Contains numbers
            "John@Doe", // Contains special chars
            long_name.as_str(),
        ];

        for name in invalid_names {
            assert!(!validate_name(name), "Name '{}' should be invalid", name);
        }
    }

    #[test]
    fn test_profile_bio_validation() {
        // Valid bios
        let valid_bio = "Software engineer with expertise in Rust, Go, and distributed systems.";
        assert!(validate_bio(valid_bio));

        let long_bio = &"a".repeat(500); // Max 500 chars
        assert!(validate_bio(long_bio));

        // Invalid bios
        let too_long_bio = &"a".repeat(501);
        assert!(!validate_bio(too_long_bio));

        // Bio with valid special characters
        let bio_with_special = "I love coding! 🚀 Working on AI/ML & blockchain.";
        assert!(validate_bio(bio_with_special));
    }

    #[test]
    fn test_profile_location_validation() {
        // Valid locations
        let valid_locations = vec![
            "San Francisco, CA",
            "New York, NY, USA",
            "London, UK",
            "Tokyo, Japan",
            "Remote",
            "São Paulo, Brazil",
            "München, Germany",
        ];

        for location in valid_locations {
            assert!(validate_location(location), "Location '{}' should be valid", location);
        }

        // Invalid locations
        let too_long_location = "a".repeat(101);
        let invalid_locations = vec![
            "", // Empty
            too_long_location.as_str(), // Too long (>100 chars)
        ];

        for location in invalid_locations {
            assert!(!validate_location(location), "Location '{}' should be invalid", location);
        }
    }

    #[test]
    fn test_profile_website_validation() {
        // Valid websites
        let valid_websites = vec![
            "https://example.com",
            "http://localhost:3000",
            "https://sub.domain.example.com",
            "https://example.com/path/to/page",
            "https://example.com?query=value",
        ];

        for website in valid_websites {
            assert!(validate_website(website), "Website '{}' should be valid", website);
        }

        // Invalid websites
        let invalid_websites = vec![
            "not-a-url",
            "ftp://example.com", // Not HTTP/HTTPS
            "javascript:alert(1)", // XSS attempt
            "example.com", // Missing protocol
            "", // Empty
        ];

        for website in invalid_websites {
            assert!(!validate_website(website), "Website '{}' should be invalid", website);
        }
    }

    #[test]
    fn test_profile_avatar_url_validation() {
        // Valid avatar URLs
        let valid_urls = vec![
            "https://example.com/avatar.jpg",
            "https://cdn.example.com/users/123/avatar.png",
            "https://gravatar.com/avatar/hash",
            "https://s3.amazonaws.com/bucket/avatar.webp",
        ];

        for url in valid_urls {
            assert!(validate_avatar_url(url), "Avatar URL '{}' should be valid", url);
        }

        // Invalid avatar URLs
        let invalid_urls = vec![
            "not-a-url",
            "http://example.com/avatar.exe", // Dangerous extension
            "javascript:void(0)", // XSS attempt
            "../../../etc/passwd", // Path traversal
        ];

        for url in invalid_urls {
            assert!(!validate_avatar_url(url), "Avatar URL '{}' should be invalid", url);
        }
    }

    #[test]
    fn test_profile_preferences_validation() {
        // Valid JSON preferences
        let valid_prefs = vec![
            r#"{"theme": "dark"}"#,
            r#"{"notifications": true, "language": "en"}"#,
            r#"{"settings": {"privacy": "public", "show_email": false}}"#,
            r#"{}"#, // Empty object
        ];

        for pref in valid_prefs {
            assert!(validate_preferences_json(pref), "Preferences '{}' should be valid", pref);
        }

        // Invalid JSON
        let invalid_prefs = vec![
            "not-json",
            r#"{"unclosed": "#,
            r#"['array', 'not', 'object']"#, // Array instead of object
            "", // Empty string
        ];

        for pref in invalid_prefs {
            assert!(!validate_preferences_json(pref), "Preferences '{}' should be invalid", pref);
        }
    }

    #[test]
    fn test_profile_full_name_generation() {
        let mut profile = create_test_profile();

        // Both names present
        assert_eq!(get_full_name(&profile), "John Doe");

        // Only first name
        profile.last_name = None;
        assert_eq!(get_full_name(&profile), "John");

        // Only last name
        profile.first_name = None;
        profile.last_name = Some("Doe".to_string());
        assert_eq!(get_full_name(&profile), "Doe");

        // No names
        profile.last_name = None;
        assert_eq!(get_full_name(&profile), "");
    }

    #[test]
    fn test_profile_display_name_generation() {
        let profile = create_test_profile();

        // Should prefer full name when available
        let display_name = get_display_name(&profile, "johndoe");
        assert_eq!(display_name, "John Doe");

        // Should fall back to username when no name
        let mut profile_no_name = profile.clone();
        profile_no_name.first_name = None;
        profile_no_name.last_name = None;
        let display_name = get_display_name(&profile_no_name, "johndoe");
        assert_eq!(display_name, "johndoe");
    }

    #[test]
    fn test_profile_completeness_score() {
        let mut profile = UserProfile {
            user_id: Uuid::new_v4(),
            first_name: None,
            last_name: None,
            bio: None,
            location: None,
            website: None,
            avatar_url: None,
            preferences: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Empty profile
        assert_eq!(calculate_completeness(&profile), 0);

        // Add fields one by one
        profile.first_name = Some("John".to_string());
        assert_eq!(calculate_completeness(&profile), 16); // ~1/6 = 16%

        profile.last_name = Some("Doe".to_string());
        assert_eq!(calculate_completeness(&profile), 33); // ~2/6 = 33%

        profile.bio = Some("Bio".to_string());
        assert_eq!(calculate_completeness(&profile), 50); // 3/6 = 50%

        profile.location = Some("Location".to_string());
        assert_eq!(calculate_completeness(&profile), 66); // ~4/6 = 66%

        profile.website = Some("https://example.com".to_string());
        assert_eq!(calculate_completeness(&profile), 83); // ~5/6 = 83%

        profile.avatar_url = Some("https://example.com/avatar.jpg".to_string());
        assert_eq!(calculate_completeness(&profile), 100); // 6/6 = 100%

        // Preferences is always present as serde_json::Value, so we get 100% with the other 6 fields
    }

    #[test]
    fn test_profile_update_timestamp() {
        let mut profile = create_test_profile();
        let initial_updated_at = profile.updated_at;

        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update profile
        profile.bio = Some("Updated bio".to_string());
        profile.updated_at = Utc::now();

        assert!(profile.updated_at > initial_updated_at);
    }

    #[test]
    fn test_profile_clone() {
        let profile1 = create_test_profile();
        let profile2 = profile1.clone();

        assert_eq!(profile1.user_id, profile2.user_id);
        assert_eq!(profile1.first_name, profile2.first_name);
        assert_eq!(profile1.last_name, profile2.last_name);
        assert_eq!(profile1.bio, profile2.bio);
    }

    #[test]
    fn test_profile_privacy_settings() {
        let preferences = r#"{
            "privacy": {
                "show_email": false,
                "show_location": true,
                "profile_visibility": "public"
            }
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(preferences).unwrap();
        assert_eq!(parsed["privacy"]["show_email"], false);
        assert_eq!(parsed["privacy"]["show_location"], true);
        assert_eq!(parsed["privacy"]["profile_visibility"], "public");
    }

    // Validation helper functions
    fn validate_name(name: &str) -> bool {
        !name.is_empty() &&
        name.len() >= 2 &&
        name.len() <= 50 &&
        !name.chars().any(|c| c.is_ascii_digit() || c == '@' || c == '#')
    }

    fn validate_bio(bio: &str) -> bool {
        bio.len() <= 500
    }

    fn validate_location(location: &str) -> bool {
        !location.is_empty() && location.len() <= 100
    }

    fn validate_website(website: &str) -> bool {
        website.starts_with("http://") || website.starts_with("https://")
    }

    fn validate_avatar_url(url: &str) -> bool {
        (url.starts_with("http://") || url.starts_with("https://")) &&
        !url.contains(".exe") &&
        !url.contains("javascript:") &&
        !url.contains("../")
    }

    fn validate_preferences_json(json: &str) -> bool {
        if json.is_empty() {
            return false;
        }

        match serde_json::from_str::<serde_json::Value>(json) {
            Ok(val) => val.is_object(),
            Err(_) => false,
        }
    }

    fn get_full_name(profile: &UserProfile) -> String {
        match (&profile.first_name, &profile.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => String::new(),
        }
    }

    fn get_display_name(profile: &UserProfile, username: &str) -> String {
        let full_name = get_full_name(profile);
        if !full_name.is_empty() {
            full_name
        } else {
            username.to_string()
        }
    }

    fn calculate_completeness(profile: &UserProfile) -> u32 {
        let mut filled_fields = 0;
        let total_fields = 6; // Not counting preferences since it's always present

        if profile.first_name.is_some() { filled_fields += 1; }
        if profile.last_name.is_some() { filled_fields += 1; }
        if profile.bio.is_some() { filled_fields += 1; }
        if profile.location.is_some() { filled_fields += 1; }
        if profile.website.is_some() { filled_fields += 1; }
        if profile.avatar_url.is_some() { filled_fields += 1; }

        (filled_fields * 100) / total_fields
    }
}