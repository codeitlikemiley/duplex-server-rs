//! Tests for Validation Functions
//!
//! This module contains comprehensive tests for validation functions used throughout
//! the application, including user data validation, format validation, and business rule validation.

#[cfg(test)]
mod validation_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use regex::Regex;

    use crate::application::services::PasswordService;
    use crate::domain::models::{User, UserProfile, UserStatus};
    use crate::errors::AppError;

    // Comprehensive validation utilities
    struct ValidationUtils;

    impl ValidationUtils {
        // Email validation
        fn validate_email(email: &str) -> Result<String, AppError> {
            let email = email.trim(); // Trim first

            if email.is_empty() {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email cannot be empty".to_string(),
                });
            }

            if email.len() > 254 {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email address too long (max 254 characters)".to_string(),
                });
            }

            // More strict email validation
            let email_regex = Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9._%+-]*[a-zA-Z0-9])?@[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?\.[a-zA-Z]{2,}$").unwrap();
            if !email_regex.is_match(email) {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Invalid email format".to_string(),
                });
            }

            // Additional validation for dots
            if email.contains("..") {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email cannot contain consecutive dots".to_string(),
                });
            }

            // Check for multiple @ symbols
            if email.matches('@').count() != 1 {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email must contain exactly one @ symbol".to_string(),
                });
            }

            let parts: Vec<&str> = email.split('@').collect();
            let local_part = parts[0];
            let domain_part = parts[1];

            // Validate local part
            if local_part.is_empty() || local_part.len() > 64 {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email local part invalid (1-64 characters)".to_string(),
                });
            }

            // Validate domain part
            if domain_part.is_empty() || domain_part.len() > 253 {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email domain invalid".to_string(),
                });
            }

            Ok(email.to_lowercase())
        }

        // Username validation
        fn validate_username(username: &str) -> Result<String, AppError> {
            if username.is_empty() {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username cannot be empty".to_string(),
                });
            }

            if username.len() < 3 {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username must be at least 3 characters long".to_string(),
                });
            }

            if username.len() > 50 {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username cannot be longer than 50 characters".to_string(),
                });
            }

            // Check for valid characters
            let username_regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
            if !username_regex.is_match(username) {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username can only contain letters, numbers, underscores, and dashes".to_string(),
                });
            }

            // Check for reserved usernames
            let reserved = vec![
                "admin", "root", "administrator", "moderator", "mod",
                "system", "api", "www", "mail", "email", "support",
                "help", "info", "contact", "sales", "marketing",
                "test", "demo", "guest", "anonymous", "null", "undefined"
            ];

            if reserved.contains(&username.to_lowercase().as_str()) {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username is reserved".to_string(),
                });
            }

            Ok(username.trim().to_string())
        }

        // Name validation (first name, last name)
        fn validate_name(name: &str, field_name: &str) -> Result<String, AppError> {
            if name.is_empty() {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} cannot be empty", field_name),
                });
            }

            if name.len() < 2 {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} must be at least 2 characters long", field_name),
                });
            }

            if name.len() > 50 {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} cannot be longer than 50 characters", field_name),
                });
            }

            // Only letters, spaces, apostrophes, and hyphens
            let name_regex = Regex::new(r"^[a-zA-ZÀ-ÿ\s'-]+$").unwrap();
            if !name_regex.is_match(name) {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} contains invalid characters", field_name),
                });
            }

            // No leading/trailing spaces
            if name.starts_with(' ') || name.ends_with(' ') {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} cannot start or end with spaces", field_name),
                });
            }

            Ok(name.trim().to_string())
        }

        // Bio validation
        fn validate_bio(bio: &str) -> Result<String, AppError> {
            if bio.len() > 500 {
                return Err(AppError::Validation {
                    field: "bio".to_string(),
                    message: "Bio cannot be longer than 500 characters".to_string(),
                });
            }

            // Check for excessive whitespace
            let cleaned = bio.trim();
            if cleaned != bio && !bio.is_empty() {
                return Err(AppError::Validation {
                    field: "bio".to_string(),
                    message: "Bio has excessive whitespace".to_string(),
                });
            }

            // Check for potential HTML/script content
            if bio.contains('<') || bio.contains('>') {
                return Err(AppError::Validation {
                    field: "bio".to_string(),
                    message: "Bio cannot contain HTML tags".to_string(),
                });
            }

            Ok(bio.to_string())
        }

        // Website URL validation
        fn validate_website_url(url: &str) -> Result<String, AppError> {
            if url.is_empty() {
                return Ok(url.to_string());
            }

            if url.len() > 1000 {
                return Err(AppError::Validation {
                    field: "website".to_string(),
                    message: "URL too long (max 1000 characters)".to_string(),
                });
            }

            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(AppError::Validation {
                    field: "website".to_string(),
                    message: "URL must start with http:// or https://".to_string(),
                });
            }

            // Check for dangerous protocols
            let dangerous_protocols = vec!["javascript:", "data:", "vbscript:", "file:", "ftp:"];
            for protocol in &dangerous_protocols {
                if url.to_lowercase().contains(protocol) {
                    return Err(AppError::Validation {
                        field: "website".to_string(),
                        message: "URL contains dangerous protocol".to_string(),
                    });
                }
            }

            // Basic URL regex validation
            let url_regex = Regex::new(r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$").unwrap();
            if !url_regex.is_match(url) {
                return Err(AppError::Validation {
                    field: "website".to_string(),
                    message: "Invalid URL format".to_string(),
                });
            }

            Ok(url.to_string())
        }

        // Location validation
        fn validate_location(location: &str) -> Result<String, AppError> {
            if location.is_empty() {
                return Ok(location.to_string());
            }

            if location.len() > 100 {
                return Err(AppError::Validation {
                    field: "location".to_string(),
                    message: "Location cannot be longer than 100 characters".to_string(),
                });
            }

            // Only letters, numbers, spaces, commas, and basic punctuation
            let location_regex = Regex::new(r"^[a-zA-ZÀ-ÿ0-9\s,.-]+$").unwrap();
            if !location_regex.is_match(location) {
                return Err(AppError::Validation {
                    field: "location".to_string(),
                    message: "Location contains invalid characters".to_string(),
                });
            }

            Ok(location.trim().to_string())
        }

        // Avatar URL validation
        fn validate_avatar_url(url: &str) -> Result<String, AppError> {
            if url.is_empty() {
                return Ok(url.to_string());
            }

            if url.len() > 2048 {
                return Err(AppError::Validation {
                    field: "avatar_url".to_string(),
                    message: "Avatar URL too long (max 2048 characters)".to_string(),
                });
            }

            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(AppError::Validation {
                    field: "avatar_url".to_string(),
                    message: "Avatar URL must start with http:// or https://".to_string(),
                });
            }

            // Check for image file extensions
            let valid_extensions = vec![".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp"];
            let has_valid_extension = valid_extensions.iter().any(|ext| {
                url.to_lowercase().contains(ext)
            });

            if !has_valid_extension {
                return Err(AppError::Validation {
                    field: "avatar_url".to_string(),
                    message: "Avatar URL must point to an image file".to_string(),
                });
            }

            // Check for dangerous file types
            let dangerous_extensions = vec![".exe", ".bat", ".cmd", ".scr", ".php", ".js"];
            for ext in &dangerous_extensions {
                if url.to_lowercase().contains(ext) {
                    return Err(AppError::Validation {
                        field: "avatar_url".to_string(),
                        message: "Avatar URL contains dangerous file type".to_string(),
                    });
                }
            }

            Ok(url.to_string())
        }

        // JSON preferences validation
        fn validate_preferences(preferences: &serde_json::Value) -> Result<(), AppError> {
            // Check JSON structure size
            let json_str = preferences.to_string();
            if json_str.len() > 10240 { // 10KB limit
                return Err(AppError::Validation {
                    field: "preferences".to_string(),
                    message: "Preferences data too large (max 10KB)".to_string(),
                });
            }

            // Ensure it's an object
            if !preferences.is_object() {
                return Err(AppError::Validation {
                    field: "preferences".to_string(),
                    message: "Preferences must be a JSON object".to_string(),
                });
            }

            // Check for dangerous content
            if json_str.contains("<script>") || json_str.contains("javascript:") {
                return Err(AppError::Validation {
                    field: "preferences".to_string(),
                    message: "Preferences contain dangerous content".to_string(),
                });
            }

            // Validate specific preference fields
            if let Some(obj) = preferences.as_object() {
                // Theme validation
                if let Some(theme) = obj.get("theme") {
                    if let Some(theme_str) = theme.as_str() {
                        let valid_themes = vec!["light", "dark", "auto"];
                        if !valid_themes.contains(&theme_str) {
                            return Err(AppError::Validation {
                                field: "preferences.theme".to_string(),
                                message: "Invalid theme value".to_string(),
                            });
                        }
                    }
                }

                // Language validation
                if let Some(language) = obj.get("language") {
                    if let Some(lang_str) = language.as_str() {
                        let language_regex = Regex::new(r"^[a-z]{2}(-[A-Z]{2})?$").unwrap();
                        if !language_regex.is_match(lang_str) {
                            return Err(AppError::Validation {
                                field: "preferences.language".to_string(),
                                message: "Invalid language code format".to_string(),
                            });
                        }
                    }
                }

                // Timezone validation
                if let Some(timezone) = obj.get("timezone") {
                    if let Some(tz_str) = timezone.as_str() {
                        // Basic timezone format validation
                        if tz_str.len() > 50 {
                            return Err(AppError::Validation {
                                field: "preferences.timezone".to_string(),
                                message: "Timezone string too long".to_string(),
                            });
                        }
                    }
                }
            }

            Ok(())
        }

        // UUID validation
        fn validate_uuid(id: &str, field_name: &str) -> Result<Uuid, AppError> {
            id.parse::<Uuid>().map_err(|_| AppError::Validation {
                field: field_name.to_string(),
                message: format!("Invalid {} format", field_name),
            })
        }

        // Date validation
        fn validate_date_in_past(date: chrono::DateTime<Utc>, field_name: &str) -> Result<(), AppError> {
            let now = Utc::now();
            if date > now {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} cannot be in the future", field_name),
                });
            }

            // Check for reasonable bounds (not too far in the past)
            let min_date = now - Duration::days(36500); // ~100 years ago
            if date < min_date {
                return Err(AppError::Validation {
                    field: field_name.to_string(),
                    message: format!("{} is too far in the past", field_name),
                });
            }

            Ok(())
        }

        // Age validation (for date of birth)
        fn validate_age(birth_date: chrono::DateTime<Utc>) -> Result<(), AppError> {
            let now = Utc::now();
            let age_years = (now - birth_date).num_days() / 365;

            if age_years < 13 {
                return Err(AppError::Validation {
                    field: "birth_date".to_string(),
                    message: "User must be at least 13 years old".to_string(),
                });
            }

            if age_years > 150 {
                return Err(AppError::Validation {
                    field: "birth_date".to_string(),
                    message: "Invalid birth date".to_string(),
                });
            }

            Ok(())
        }

        // User status validation
        fn validate_user_status_transition(from: UserStatus, to: UserStatus) -> Result<(), AppError> {
            use UserStatus::*;

            let valid_transitions = match from {
                Active => vec![Inactive, Suspended],
                Inactive => vec![Active, Suspended, PendingVerification],
                Suspended => vec![Active, Inactive],
                PendingVerification => vec![Active, Inactive],
            };

            if !valid_transitions.contains(&to) {
                return Err(AppError::Validation {
                    field: "status".to_string(),
                    message: format!("Invalid status transition from {:?} to {:?}", from, to),
                });
            }

            Ok(())
        }

        // Rate limiting validation
        fn validate_rate_limit_params(max_requests: u32, window_seconds: u64) -> Result<(), AppError> {
            if max_requests == 0 {
                return Err(AppError::Validation {
                    field: "max_requests".to_string(),
                    message: "Max requests must be greater than 0".to_string(),
                });
            }

            if max_requests > 10000 {
                return Err(AppError::Validation {
                    field: "max_requests".to_string(),
                    message: "Max requests too high (limit: 10000)".to_string(),
                });
            }

            if window_seconds == 0 {
                return Err(AppError::Validation {
                    field: "window_seconds".to_string(),
                    message: "Window seconds must be greater than 0".to_string(),
                });
            }

            if window_seconds > 86400 { // 24 hours
                return Err(AppError::Validation {
                    field: "window_seconds".to_string(),
                    message: "Window too long (max: 24 hours)".to_string(),
                });
            }

            Ok(())
        }

        // Session validation
        fn validate_session_duration(duration_hours: u32) -> Result<(), AppError> {
            if duration_hours == 0 {
                return Err(AppError::Validation {
                    field: "session_duration".to_string(),
                    message: "Session duration must be greater than 0".to_string(),
                });
            }

            if duration_hours > 8760 { // 1 year
                return Err(AppError::Validation {
                    field: "session_duration".to_string(),
                    message: "Session duration too long (max: 1 year)".to_string(),
                });
            }

            Ok(())
        }
    }

    // Tests for Email Validation
    #[test]
    fn test_email_validation_valid_emails() {
        let valid_emails = vec![
            "test@example.com",
            "user.name@domain.co.uk",
            "user+tag@example.org",
            "firstname.lastname@company.com",
            "user123@test-domain.net",
        ];

        for email in valid_emails {
            assert!(ValidationUtils::validate_email(email).is_ok(), "Email should be valid: {}", email);
        }
    }

    #[test]
    fn test_email_validation_invalid_emails() {
        let long_local = format!("{}@example.com", "a".repeat(65));
        let long_domain = format!("test@{}.com", "a".repeat(254));
        let invalid_emails = vec![
            "",
            "invalid-email",
            "@example.com",
            "test@",
            "test@@example.com",
            "test@.com",
            "test@com",
            "test.@example.com",
            ".test@example.com",
            "test@example.",
            &long_local, // Local part too long
            &long_domain, // Domain too long
            "test@exam ple.com", // Space in domain
            "te st@example.com", // Space in local
        ];

        for email in invalid_emails {
            assert!(ValidationUtils::validate_email(&email).is_err(), "Email should be invalid: {}", email);
        }
    }

    #[test]
    fn test_email_normalization() {
        let test_cases = vec![
            ("TEST@EXAMPLE.COM", "test@example.com"),
            ("  test@example.com  ", "test@example.com"),
        ];

        for (input, expected) in test_cases {
            let result = ValidationUtils::validate_email(input);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    // Tests for Username Validation
    #[test]
    fn test_username_validation_valid() {
        let valid_usernames = vec![
            "user123",
            "test_user",
            "user-name",
            "Username1",
            "a_b-c123",
        ];

        for username in valid_usernames {
            assert!(ValidationUtils::validate_username(username).is_ok(), "Username should be valid: {}", username);
        }
    }

    #[test]
    fn test_username_validation_invalid() {
        let long_username = "a".repeat(51);
        let invalid_usernames = vec![
            "",
            "ab", // Too short
            &long_username, // Too long
            "user@domain",
            "user name", // Space
            "user.name", // Dot
            "user#name", // Special char
            "admin", // Reserved
            "root", // Reserved
            "ADMINISTRATOR", // Reserved (case insensitive)
        ];

        for username in invalid_usernames {
            assert!(ValidationUtils::validate_username(username).is_err(), "Username should be invalid: {}", username);
        }
    }

    // Tests for Name Validation
    #[test]
    fn test_name_validation_valid() {
        let valid_names = vec![
            "John",
            "Mary-Jane",
            "O'Connor",
            "José",
            "François",
            "Li Wei",
        ];

        for name in valid_names {
            assert!(ValidationUtils::validate_name(name, "first_name").is_ok(), "Name should be valid: {}", name);
        }
    }

    #[test]
    fn test_name_validation_invalid() {
        let long_name = "a".repeat(51);
        let invalid_names = vec![
            "",
            "A", // Too short
            &long_name, // Too long
            " John", // Leading space
            "John ", // Trailing space
            "John123", // Numbers
            "John@Smith", // Special chars
            "John<script>", // HTML
        ];

        for name in invalid_names {
            assert!(ValidationUtils::validate_name(name, "first_name").is_err(), "Name should be invalid: {}", name);
        }
    }

    // Tests for Bio Validation
    #[test]
    fn test_bio_validation() {
        // Valid bio
        let valid_bio = "Software developer passionate about open source.";
        assert!(ValidationUtils::validate_bio(valid_bio).is_ok());

        // Empty bio (should be valid)
        assert!(ValidationUtils::validate_bio("").is_ok());

        // Bio too long
        let long_bio = "a".repeat(501);
        assert!(ValidationUtils::validate_bio(&long_bio).is_err());

        // Bio with HTML
        let html_bio = "Developer <script>alert('xss')</script>";
        assert!(ValidationUtils::validate_bio(html_bio).is_err());

        // Bio with excessive whitespace
        let whitespace_bio = "  Developer  ";
        assert!(ValidationUtils::validate_bio(whitespace_bio).is_err());
    }

    // Tests for Website URL Validation
    #[test]
    fn test_website_url_validation_valid() {
        let valid_urls = vec![
            "",
            "https://example.com",
            "http://example.com",
            "https://www.example.com/path?query=value",
            "https://subdomain.example.com",
        ];

        for url in valid_urls {
            assert!(ValidationUtils::validate_website_url(url).is_ok(), "URL should be valid: {}", url);
        }
    }

    #[test]
    fn test_website_url_validation_invalid() {
        let long_url = format!("https://example.com/{}", "a".repeat(2000));
        let invalid_urls = vec![
            "example.com", // No protocol
            "ftp://example.com", // Invalid protocol
            "javascript:alert('xss')", // Dangerous protocol
            "https://", // Incomplete URL
            &long_url, // Too long
        ];

        for url in invalid_urls {
            assert!(ValidationUtils::validate_website_url(url).is_err(), "URL should be invalid: {}", url);
        }
    }

    // Tests for Avatar URL Validation
    #[test]
    fn test_avatar_url_validation() {
        // Valid avatar URLs
        let valid_urls = vec![
            "",
            "https://example.com/avatar.jpg",
            "https://example.com/profile.png",
            "http://example.com/image.gif",
        ];

        for url in valid_urls {
            assert!(ValidationUtils::validate_avatar_url(url).is_ok(), "Avatar URL should be valid: {}", url);
        }

        // Invalid avatar URLs
        let invalid_urls = vec![
            "https://example.com/file.exe", // Dangerous extension
            "https://example.com/script.php", // Server script
            "https://example.com/file.txt", // Not an image
            "avatar.jpg", // No protocol
        ];

        for url in invalid_urls {
            assert!(ValidationUtils::validate_avatar_url(url).is_err(), "Avatar URL should be invalid: {}", url);
        }
    }

    // Tests for Preferences Validation
    #[test]
    fn test_preferences_validation() {
        // Valid preferences
        let valid_prefs = json!({
            "theme": "dark",
            "language": "en-US",
            "notifications": true,
            "timezone": "America/New_York"
        });
        assert!(ValidationUtils::validate_preferences(&valid_prefs).is_ok());

        // Invalid theme
        let invalid_theme = json!({
            "theme": "rainbow"
        });
        assert!(ValidationUtils::validate_preferences(&invalid_theme).is_err());

        // Invalid language code
        let invalid_lang = json!({
            "language": "english"
        });
        assert!(ValidationUtils::validate_preferences(&invalid_lang).is_err());

        // Not an object
        let not_object = json!("string");
        assert!(ValidationUtils::validate_preferences(&not_object).is_err());

        // Too large
        let large_prefs = json!({
            "data": "a".repeat(10241)
        });
        assert!(ValidationUtils::validate_preferences(&large_prefs).is_err());

        // Dangerous content
        let dangerous = json!({
            "script": "<script>alert('xss')</script>"
        });
        assert!(ValidationUtils::validate_preferences(&dangerous).is_err());
    }

    // Tests for UUID Validation
    #[test]
    fn test_uuid_validation() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(ValidationUtils::validate_uuid(valid_uuid, "user_id").is_ok());

        let invalid_uuids = vec![
            "",
            "not-a-uuid",
            "550e8400-e29b-41d4-a716", // Too short
            "550e8400-e29b-41d4-a716-446655440000-extra", // Too long
        ];

        for uuid in invalid_uuids {
            assert!(ValidationUtils::validate_uuid(uuid, "user_id").is_err(), "UUID should be invalid: {}", uuid);
        }
    }

    // Tests for Date Validation
    #[test]
    fn test_date_validation() {
        let now = Utc::now();

        // Valid past date
        let past_date = now - Duration::days(365);
        assert!(ValidationUtils::validate_date_in_past(past_date, "created_at").is_ok());

        // Invalid future date
        let future_date = now + Duration::days(1);
        assert!(ValidationUtils::validate_date_in_past(future_date, "created_at").is_err());

        // Invalid too far past
        let too_old = now - Duration::days(40000);
        assert!(ValidationUtils::validate_date_in_past(too_old, "created_at").is_err());
    }

    // Tests for Age Validation
    #[test]
    fn test_age_validation() {
        let now = Utc::now();

        // Valid age (25 years old)
        let valid_birth = now - Duration::days(25 * 365);
        assert!(ValidationUtils::validate_age(valid_birth).is_ok());

        // Invalid age (too young - 10 years old)
        let too_young = now - Duration::days(10 * 365);
        assert!(ValidationUtils::validate_age(too_young).is_err());

        // Invalid age (too old - 200 years old)
        let too_old = now - Duration::days(200 * 365);
        assert!(ValidationUtils::validate_age(too_old).is_err());
    }

    // Tests for User Status Transition Validation
    #[test]
    fn test_user_status_transitions() {
        use UserStatus::*;

        // Valid transitions
        assert!(ValidationUtils::validate_user_status_transition(Active, Inactive).is_ok());
        assert!(ValidationUtils::validate_user_status_transition(Active, Suspended).is_ok());
        assert!(ValidationUtils::validate_user_status_transition(Suspended, Active).is_ok());

        // Invalid transitions
        assert!(ValidationUtils::validate_user_status_transition(PendingVerification, Suspended).is_err());
        assert!(ValidationUtils::validate_user_status_transition(Suspended, PendingVerification).is_err());
    }

    // Tests for Rate Limiting Validation
    #[test]
    fn test_rate_limit_validation() {
        // Valid parameters
        assert!(ValidationUtils::validate_rate_limit_params(100, 3600).is_ok());
        assert!(ValidationUtils::validate_rate_limit_params(1, 1).is_ok());

        // Invalid parameters
        assert!(ValidationUtils::validate_rate_limit_params(0, 3600).is_err()); // Zero requests
        assert!(ValidationUtils::validate_rate_limit_params(20000, 3600).is_err()); // Too many requests
        assert!(ValidationUtils::validate_rate_limit_params(100, 0).is_err()); // Zero window
        assert!(ValidationUtils::validate_rate_limit_params(100, 100000).is_err()); // Window too long
    }

    // Tests for Session Duration Validation
    #[test]
    fn test_session_duration_validation() {
        // Valid durations
        assert!(ValidationUtils::validate_session_duration(1).is_ok()); // 1 hour
        assert!(ValidationUtils::validate_session_duration(24).is_ok()); // 1 day
        assert!(ValidationUtils::validate_session_duration(168).is_ok()); // 1 week

        // Invalid durations
        assert!(ValidationUtils::validate_session_duration(0).is_err()); // Zero duration
        assert!(ValidationUtils::validate_session_duration(10000).is_err()); // Too long
    }

    // Integration test with password service
    #[test]
    fn test_password_validation_integration() {
        // Valid passwords
        let valid_passwords = vec![
            "StrongP@ss123",
            "MySecure!Pass1",
            "Complex#Password2024",
        ];

        for password in valid_passwords {
            assert!(PasswordService::validate_password_strength(password).is_ok(),
                "Password should be valid: {}", password);
        }

        // Invalid passwords
        let invalid_passwords = vec![
            "short",
            "alllowercase123!",
            "ALLUPPERCASE123!",
            "NoSpecialChars123",
            "NoNumbers!",
            "noupper123!",
        ];

        for password in invalid_passwords {
            assert!(PasswordService::validate_password_strength(password).is_err(),
                "Password should be invalid: {}", password);
        }
    }

    // Comprehensive validation test
    #[test]
    fn test_user_profile_comprehensive_validation() {
        // Test with valid data
        let email = "test@example.com";
        let username = "testuser123";
        let first_name = "John";
        let last_name = "Doe";
        let bio = "Software developer";
        let website = "https://johndoe.com";
        let location = "San Francisco, CA";
        let avatar_url = "https://example.com/avatar.jpg";
        let preferences = json!({
            "theme": "dark",
            "language": "en"
        });

        // All should pass
        assert!(ValidationUtils::validate_email(email).is_ok());
        assert!(ValidationUtils::validate_username(username).is_ok());
        assert!(ValidationUtils::validate_name(first_name, "first_name").is_ok());
        assert!(ValidationUtils::validate_name(last_name, "last_name").is_ok());
        assert!(ValidationUtils::validate_bio(bio).is_ok());
        assert!(ValidationUtils::validate_website_url(website).is_ok());
        assert!(ValidationUtils::validate_location(location).is_ok());
        assert!(ValidationUtils::validate_avatar_url(avatar_url).is_ok());
        assert!(ValidationUtils::validate_preferences(&preferences).is_ok());
    }

    // Edge case and boundary testing
    #[test]
    fn test_validation_edge_cases() {
        let max_length = "a".repeat(50);
        let over_max_length = "a".repeat(51);

        // Test exact boundary lengths
        assert!(ValidationUtils::validate_username("abc").is_ok()); // Minimum length
        assert!(ValidationUtils::validate_username(&max_length).is_ok()); // Maximum length
        assert!(ValidationUtils::validate_username("ab").is_err()); // Below minimum
        assert!(ValidationUtils::validate_username(&over_max_length).is_err()); // Above maximum

        // Test unicode handling
        assert!(ValidationUtils::validate_name("José", "first_name").is_ok());
        assert!(ValidationUtils::validate_name("François", "first_name").is_ok());
        assert!(ValidationUtils::validate_location("Montréal, QC").is_ok());

        // Test empty/null cases
        assert!(ValidationUtils::validate_website_url("").is_ok()); // Empty URL is valid
        assert!(ValidationUtils::validate_location("").is_ok()); // Empty location is valid
        assert!(ValidationUtils::validate_bio("").is_ok()); // Empty bio is valid
    }
}