//! Tests for Security Utilities
//!
//! This module contains comprehensive tests for security utilities,
//! including JWT operations, password validation, input sanitization, and security middleware.

#[cfg(test)]
mod security_utils_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use jsonwebtoken::{Algorithm, Validation};
    use serde_json::json;
    use std::collections::HashMap;

    use crate::infrastructure::auth::{JwtService, Claims};
    use crate::application::services::PasswordService;
    use crate::errors::AppError;

    // Mock security utilities for testing
    struct MockSecurityUtils;

    impl MockSecurityUtils {
        // Input validation utilities
        fn validate_email(email: &str) -> Result<String, AppError> {
            if email.is_empty() {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Email cannot be empty".to_string(),
                });
            }

            let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
            if !email_regex.is_match(email) {
                return Err(AppError::Validation {
                    field: "email".to_string(),
                    message: "Invalid email format".to_string(),
                });
            }

            Ok(email.to_lowercase().trim().to_string())
        }

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

            let username_regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
            if !username_regex.is_match(username) {
                return Err(AppError::Validation {
                    field: "username".to_string(),
                    message: "Username can only contain letters, numbers, underscores, and dashes".to_string(),
                });
            }

            Ok(username.trim().to_string())
        }

        // Input sanitization utilities
        fn sanitize_html(input: &str) -> String {
            // Basic HTML sanitization - remove script tags and dangerous attributes
            let mut sanitized = input.to_string();

            // Remove script tags
            let script_regex = regex::Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap();
            sanitized = script_regex.replace_all(&sanitized, "").to_string();

            // Remove javascript: URLs
            let js_regex = regex::Regex::new(r"(?i)javascript:").unwrap();
            sanitized = js_regex.replace_all(&sanitized, "").to_string();

            // Remove on* event attributes
            let event_regex = regex::Regex::new(r#"(?i)\s+on\w+\s*=\s*['"][^'"]*['"]"#).unwrap();
            sanitized = event_regex.replace_all(&sanitized, "").to_string();

            sanitized
        }

        fn sanitize_sql(input: &str) -> String {
            // Basic SQL injection prevention
            input
                .replace("'", "''")  // Escape single quotes
                .replace("--", "")   // Remove SQL comments
                .replace(";", "")    // Remove statement terminators
        }

        // Security header utilities
        fn generate_csrf_token() -> String {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let token: [u8; 32] = rng.random();
            base64::encode(token)
        }

        fn validate_csrf_token(token: &str, expected: &str) -> bool {
            token == expected && !token.is_empty()
        }

        // Rate limiting utilities
        fn check_rate_limit(key: &str, max_requests: u32, window_seconds: u64, requests: &mut HashMap<String, Vec<chrono::DateTime<Utc>>>) -> bool {
            let now = Utc::now();
            let window_start = now - Duration::seconds(window_seconds as i64);

            // Clean old requests
            requests.entry(key.to_string()).or_insert_with(Vec::new)
                .retain(|&timestamp| timestamp > window_start);

            let request_count = requests.get(key).unwrap().len();

            if request_count >= max_requests as usize {
                false // Rate limit exceeded
            } else {
                requests.get_mut(key).unwrap().push(now);
                true // Request allowed
            }
        }

        // Session security utilities
        fn generate_session_id() -> String {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let id: [u8; 32] = rng.random();
            id.iter().map(|byte| format!("{:02x}", byte)).collect()
        }

        fn validate_session_id(session_id: &str) -> bool {
            // Session ID should be 64 hex characters
            session_id.len() == 64 && session_id.chars().all(|c| c.is_ascii_hexdigit())
        }

        // Password utilities (enhanced)
        fn estimate_password_entropy(password: &str) -> f64 {
            let mut charset_size = 0;

            if password.chars().any(|c| c.is_ascii_lowercase()) {
                charset_size += 26;
            }
            if password.chars().any(|c| c.is_ascii_uppercase()) {
                charset_size += 26;
            }
            if password.chars().any(|c| c.is_ascii_digit()) {
                charset_size += 10;
            }
            if password.chars().any(|c| !c.is_alphanumeric()) {
                charset_size += 32; // Special characters
            }

            if charset_size == 0 {
                return 0.0;
            }

            let length = password.len() as f64;
            length * (charset_size as f64).log2()
        }

        fn check_common_password(password: &str) -> bool {
            // Check against common passwords
            let common_passwords = vec![
                "password", "123456", "123456789", "qwerty", "abc123",
                "password123", "admin", "letmein", "welcome", "monkey",
                "dragon", "master", "shadow", "sunshine", "football"
            ];

            common_passwords.contains(&password.to_lowercase().as_str())
        }

        // Timing attack prevention
        fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
            if a.len() != b.len() {
                return false;
            }

            let mut result = 0u8;
            for (ai, bi) in a.iter().zip(b.iter()) {
                result |= ai ^ bi;
            }

            result == 0
        }
    }

    #[test]
    fn test_jwt_service_creation() {
        let jwt_service = JwtService::new("test-secret-key");

        // Test token generation
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = jwt_service.generate_token(user_id, email);
        assert!(token.is_ok());

        let token_str = token.unwrap();
        assert!(!token_str.is_empty());
        assert!(token_str.contains("."));
    }

    #[test]
    fn test_jwt_token_verification() {
        let jwt_service = JwtService::new("test-secret-key");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        // Generate token
        let token = jwt_service.generate_token(user_id, email).unwrap();

        // Verify token
        let claims = jwt_service.verify_token(&token);
        assert!(claims.is_ok());

        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
        assert!(claims.exp > (Utc::now().timestamp() as usize));
    }

    #[test]
    fn test_jwt_invalid_token() {
        let jwt_service = JwtService::new("test-secret-key");

        // Test with invalid tokens
        assert!(jwt_service.verify_token("invalid-token").is_err());
        assert!(jwt_service.verify_token("").is_err());
        assert!(jwt_service.verify_token("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9").is_err());
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let jwt_service1 = JwtService::new("secret1");
        let jwt_service2 = JwtService::new("secret2");

        let user_id = Uuid::new_v4();
        let token = jwt_service1.generate_token(user_id, "test@example.com").unwrap();

        // Token should not verify with different secret
        assert!(jwt_service2.verify_token(&token).is_err());
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid passwords
        assert!(PasswordService::validate_password_strength("StrongP@ss123").is_ok());
        assert!(PasswordService::validate_password_strength("MySecure!Pass1").is_ok());

        // Invalid passwords
        assert!(PasswordService::validate_password_strength("short").is_err());
        assert!(PasswordService::validate_password_strength("alllowercase").is_err());
        assert!(PasswordService::validate_password_strength("ALLUPPERCASE").is_err());
        assert!(PasswordService::validate_password_strength("NoSpecialChars123").is_err());
        assert!(PasswordService::validate_password_strength("NoNumbers!").is_err());
    }

    #[test]
    fn test_email_validation() {
        // Valid emails
        assert!(MockSecurityUtils::validate_email("test@example.com").is_ok());
        assert!(MockSecurityUtils::validate_email("user.name@domain.co.uk").is_ok());
        assert!(MockSecurityUtils::validate_email("user+tag@example.org").is_ok());

        // Invalid emails
        assert!(MockSecurityUtils::validate_email("").is_err());
        assert!(MockSecurityUtils::validate_email("invalid-email").is_err());
        assert!(MockSecurityUtils::validate_email("@example.com").is_err());
        assert!(MockSecurityUtils::validate_email("test@").is_err());
        assert!(MockSecurityUtils::validate_email("test@.com").is_err());
    }

    #[test]
    fn test_email_normalization() {
        let result = MockSecurityUtils::validate_email("TEST@EXAMPLE.COM");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");
    }

    #[test]
    fn test_username_validation() {
        // Valid usernames
        assert!(MockSecurityUtils::validate_username("user123").is_ok());
        assert!(MockSecurityUtils::validate_username("test_user").is_ok());
        assert!(MockSecurityUtils::validate_username("user-name").is_ok());

        // Invalid usernames
        assert!(MockSecurityUtils::validate_username("").is_err());
        assert!(MockSecurityUtils::validate_username("ab").is_err()); // Too short
        assert!(MockSecurityUtils::validate_username(&"a".repeat(51)).is_err()); // Too long
        assert!(MockSecurityUtils::validate_username("user@domain").is_err()); // Invalid chars
        assert!(MockSecurityUtils::validate_username("user name").is_err()); // Space
    }

    #[test]
    fn test_html_sanitization() {
        // Test script tag removal
        let malicious_html = r#"<div>Hello <script>alert('xss')</script> World</div>"#;
        let sanitized = MockSecurityUtils::sanitize_html(malicious_html);
        assert!(!sanitized.contains("<script>"));
        assert!(!sanitized.contains("alert('xss')"));

        // Test javascript: URL removal
        let js_url = r#"<a href="javascript:alert('xss')">Click</a>"#;
        let sanitized = MockSecurityUtils::sanitize_html(js_url);
        assert!(!sanitized.contains("javascript:"));

        // Test event attribute removal
        let event_attrs = r#"<div onclick="alert('xss')" onload="badStuff()">Content</div>"#;
        let sanitized = MockSecurityUtils::sanitize_html(event_attrs);
        assert!(!sanitized.contains("onclick"));
        assert!(!sanitized.contains("onload"));
    }

    #[test]
    fn test_sql_sanitization() {
        let malicious_sql = "'; DROP TABLE users; --";
        let sanitized = MockSecurityUtils::sanitize_sql(malicious_sql);

        assert!(!sanitized.contains("--"));
        assert!(!sanitized.contains(";"));
        assert!(sanitized.contains("''"));
    }

    #[test]
    fn test_csrf_token_generation() {
        let token1 = MockSecurityUtils::generate_csrf_token();
        let token2 = MockSecurityUtils::generate_csrf_token();

        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        assert_ne!(token1, token2); // Should be unique

        // Should be valid base64
        assert!(base64::decode(&token1).is_ok());
    }

    #[test]
    fn test_csrf_token_validation() {
        let token = MockSecurityUtils::generate_csrf_token();

        assert!(MockSecurityUtils::validate_csrf_token(&token, &token));
        assert!(!MockSecurityUtils::validate_csrf_token(&token, "wrong-token"));
        assert!(!MockSecurityUtils::validate_csrf_token("", ""));
    }

    #[test]
    fn test_rate_limiting() {
        let mut requests = HashMap::new();
        let key = "user1";

        // Should allow initial requests
        for _ in 0..5 {
            assert!(MockSecurityUtils::check_rate_limit(key, 5, 60, &mut requests));
        }

        // Should deny 6th request
        assert!(!MockSecurityUtils::check_rate_limit(key, 5, 60, &mut requests));

        // Different user should be allowed
        assert!(MockSecurityUtils::check_rate_limit("user2", 5, 60, &mut requests));
    }

    #[test]
    fn test_session_id_generation() {
        let session1 = MockSecurityUtils::generate_session_id();
        let session2 = MockSecurityUtils::generate_session_id();

        assert_eq!(session1.len(), 64);
        assert_eq!(session2.len(), 64);
        assert_ne!(session1, session2);

        // Should be valid hex
        assert!(session1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(session2.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_session_id_validation() {
        let valid_session = MockSecurityUtils::generate_session_id();
        assert!(MockSecurityUtils::validate_session_id(&valid_session));

        // Invalid sessions
        assert!(!MockSecurityUtils::validate_session_id("too-short"));
        assert!(!MockSecurityUtils::validate_session_id(&"a".repeat(65))); // Too long
        assert!(!MockSecurityUtils::validate_session_id("invalid-chars-xyz")); // Non-hex chars
        assert!(!MockSecurityUtils::validate_session_id(""));
    }

    #[test]
    fn test_password_entropy_estimation() {
        // Weak password
        let weak_entropy = MockSecurityUtils::estimate_password_entropy("password");
        assert!(weak_entropy < 40.0);

        // Strong password
        let strong_entropy = MockSecurityUtils::estimate_password_entropy("Str0ng!P@ssw0rd");
        assert!(strong_entropy > 60.0);

        // Very strong password
        let very_strong_entropy = MockSecurityUtils::estimate_password_entropy("C0mpl3x!P@ssw0rd#2023");
        assert!(very_strong_entropy > 80.0);
    }

    #[test]
    fn test_common_password_detection() {
        // Common passwords should be detected
        assert!(MockSecurityUtils::check_common_password("password"));
        assert!(MockSecurityUtils::check_common_password("123456"));
        assert!(MockSecurityUtils::check_common_password("qwerty"));
        assert!(MockSecurityUtils::check_common_password("PASSWORD")); // Case insensitive

        // Unique passwords should not be detected
        assert!(!MockSecurityUtils::check_common_password("Unique!P@ssw0rd123"));
        assert!(!MockSecurityUtils::check_common_password("MySpecialPassword2023"));
    }

    #[test]
    fn test_constant_time_compare() {
        let data1 = b"secret_data";
        let data2 = b"secret_data";
        let data3 = b"different_data";

        // Same data should match
        assert!(MockSecurityUtils::constant_time_compare(data1, data2));

        // Different data should not match
        assert!(!MockSecurityUtils::constant_time_compare(data1, data3));

        // Different lengths should not match
        assert!(!MockSecurityUtils::constant_time_compare(b"short", b"longer_string"));
    }

    #[test]
    fn test_security_headers() {
        // Test HTML sanitization scenarios
        let html_test_cases = vec![
            ("empty_string", "", false),
            ("normal_input", "Hello, World!", false),
            ("script_injection", "<script>alert('xss')</script>", true),
            ("mixed_content", "Normal text <script>bad()</script> more text", true),
        ];

        for (name, input, should_change) in html_test_cases {
            let sanitized_html = MockSecurityUtils::sanitize_html(input);

            if should_change {
                assert_ne!(sanitized_html, input, "HTML sanitization failed for: {}", name);
            }

            // Sanitized HTML should never contain dangerous patterns
            assert!(!sanitized_html.contains("<script>"), "Script tag found in: {}", name);
            assert!(!sanitized_html.contains("javascript:"), "JS URL found in: {}", name);
        }

        // Test SQL sanitization scenarios
        let sql_test_cases = vec![
            ("empty_string", "", false),
            ("normal_input", "Hello, World!", false),
            ("sql_injection", "'; DROP TABLE users; --", true),
            ("single_quote", "O'Reilly", true),
        ];

        for (name, input, should_change) in sql_test_cases {
            let sanitized_sql = MockSecurityUtils::sanitize_sql(input);

            if should_change {
                assert_ne!(sanitized_sql, input, "SQL sanitization failed for: {}", name);
            }

            // Sanitized SQL should never contain dangerous patterns
            assert!(!sanitized_sql.contains("--"), "SQL comment found in: {}", name);
            assert!(!sanitized_sql.contains(";"), "SQL terminator found in: {}", name);
        }
    }

    #[test]
    fn test_edge_cases() {
        // Empty inputs
        assert!(MockSecurityUtils::validate_email("").is_err());
        assert!(MockSecurityUtils::validate_username("").is_err());

        // Very long inputs
        let long_string = "a".repeat(1000);
        let sanitized_html = MockSecurityUtils::sanitize_html(&long_string);
        assert_eq!(sanitized_html.len(), 1000);

        // Unicode inputs
        let unicode_input = "Hello 世界 🌍";
        let sanitized = MockSecurityUtils::sanitize_html(unicode_input);
        assert!(sanitized.contains("世界"));
        assert!(sanitized.contains("🌍"));

        // Null bytes (should be handled safely)
        let null_input = "test\x00data";
        let sanitized = MockSecurityUtils::sanitize_html(null_input);
        assert!(sanitized.contains("test"));
        assert!(sanitized.contains("data"));
    }

    #[test]
    fn test_multiple_security_layers() {
        let malicious_input = r#"<script>fetch('javascript:alert("xss")');</script>"#;

        // Apply multiple security layers
        let step1 = MockSecurityUtils::sanitize_html(malicious_input);
        let step2 = MockSecurityUtils::sanitize_sql(&step1);

        // Should be safe after multiple sanitizations
        assert!(!step2.contains("<script>"));
        assert!(!step2.contains("javascript:"));
        assert!(!step2.contains("--"));
    }
}