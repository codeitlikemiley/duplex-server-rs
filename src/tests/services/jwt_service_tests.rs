//! Tests for JWT Token Service
//!
//! This module contains comprehensive tests for JWT token operations,
//! including generation, validation, and security features.

#[cfg(test)]
mod jwt_service_tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    use crate::errors::AppError;

    // Test JWT claims structure
    #[derive(Debug, Serialize, Deserialize)]
    struct TestClaims {
        sub: String,        // Subject (user ID)
        exp: i64,           // Expiration time
        iat: i64,           // Issued at
        iss: String,        // Issuer
        aud: String,        // Audience
        user_id: Uuid,
        username: String,
        email: String,
        roles: Vec<String>,
    }

    // Helper struct for JWT service testing
    struct TestJwtService {
        secret: String,
        issuer: String,
        audience: String,
    }

    impl TestJwtService {
        fn new() -> Self {
            Self {
                secret: "test-secret-key-for-jwt-testing-must-be-long-enough".to_string(),
                issuer: "quake-app".to_string(),
                audience: "quake-users".to_string(),
            }
        }

        fn generate_token(&self, user_id: Uuid, username: &str, email: &str, roles: Vec<String>, expires_in: Duration) -> Result<String, AppError> {
            let now = Utc::now();
            let expires_at = now + expires_in;

            let claims = TestClaims {
                sub: user_id.to_string(),
                exp: expires_at.timestamp(),
                iat: now.timestamp(),
                iss: self.issuer.clone(),
                aud: self.audience.clone(),
                user_id,
                username: username.to_string(),
                email: email.to_string(),
                roles,
            };

            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(self.secret.as_ref()),
            )
            .map_err(|e| AppError::Internal {
                message: format!("Failed to generate JWT token: {}", e),
            })
        }

        fn validate_token(&self, token: &str) -> Result<TestClaims, AppError> {
            let mut validation = Validation::new(Algorithm::HS256);
            validation.set_audience(&[&self.audience]);
            validation.set_issuer(&[&self.issuer]);

            decode::<TestClaims>(
                token,
                &DecodingKey::from_secret(self.secret.as_ref()),
                &validation,
            )
            .map(|token_data| token_data.claims)
            .map_err(|e| AppError::Authentication {
                message: format!("Invalid JWT token: {}", e),
            })
        }

        fn extract_user_id(&self, token: &str) -> Result<Uuid, AppError> {
            let claims = self.validate_token(token)?;
            Ok(claims.user_id)
        }

        fn is_token_expired(&self, token: &str) -> Result<bool, AppError> {
            let claims = self.validate_token(token)?;
            let now = Utc::now().timestamp();
            Ok(claims.exp < now)
        }

        fn refresh_token(&self, old_token: &str, new_expires_in: Duration) -> Result<String, AppError> {
            let claims = self.validate_token(old_token)?;

            // Generate new token with same user data but new expiration
            self.generate_token(
                claims.user_id,
                &claims.username,
                &claims.email,
                claims.roles,
                new_expires_in,
            )
        }
    }

    #[test]
    fn test_jwt_token_generation() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let email = "test@example.com";
        let roles = vec!["user".to_string()];
        let expires_in = Duration::hours(1);

        let token = jwt_service.generate_token(user_id, username, email, roles, expires_in);

        assert!(token.is_ok());
        let token = token.unwrap();

        // JWT tokens have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Token should not be empty
        assert!(!token.is_empty());
        assert!(token.len() > 100); // JWT tokens are typically long
    }

    #[test]
    fn test_jwt_token_validation_success() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let email = "test@example.com";
        let roles = vec!["user".to_string(), "admin".to_string()];

        let token = jwt_service
            .generate_token(user_id, username, email, roles.clone(), Duration::hours(1))
            .unwrap();

        let claims = jwt_service.validate_token(&token);

        assert!(claims.is_ok());
        let claims = claims.unwrap();
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.email, email);
        assert_eq!(claims.roles, roles);
        assert_eq!(claims.iss, "quake-app");
        assert_eq!(claims.aud, "quake-users");
    }

    #[test]
    fn test_jwt_token_validation_failure() {
        let jwt_service = TestJwtService::new();

        let invalid_tokens = vec![
            "invalid.token.here",
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid.signature",
            "",
            "not-a-jwt-token",
            "a.b.c.d", // Too many parts
        ];

        for invalid_token in invalid_tokens {
            let result = jwt_service.validate_token(invalid_token);
            assert!(result.is_err(), "Token '{}' should be invalid", invalid_token);
        }
    }

    #[test]
    fn test_jwt_token_expiration() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        // Create token that expires in 1 second
        let short_token = jwt_service
            .generate_token(user_id, "test", "test@example.com", vec![], Duration::seconds(1))
            .unwrap();

        // Token should be valid initially
        assert!(jwt_service.validate_token(&short_token).is_ok());
        assert!(!jwt_service.is_token_expired(&short_token).unwrap());

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Token should now be expired
        assert!(jwt_service.is_token_expired(&short_token).unwrap());
    }

    #[test]
    fn test_jwt_token_user_extraction() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        let token = jwt_service
            .generate_token(user_id, "test", "test@example.com", vec![], Duration::hours(1))
            .unwrap();

        let extracted_user_id = jwt_service.extract_user_id(&token);

        assert!(extracted_user_id.is_ok());
        assert_eq!(extracted_user_id.unwrap(), user_id);
    }

    #[test]
    fn test_jwt_token_refresh() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let email = "test@example.com";
        let roles = vec!["user".to_string()];

        // Generate original token
        let original_token = jwt_service
            .generate_token(user_id, username, email, roles.clone(), Duration::hours(1))
            .unwrap();

        // Refresh token with new expiration
        let refreshed_token = jwt_service
            .refresh_token(&original_token, Duration::hours(2))
            .unwrap();

        // Both tokens should be different
        assert_ne!(original_token, refreshed_token);

        // Both should validate successfully
        assert!(jwt_service.validate_token(&original_token).is_ok());
        assert!(jwt_service.validate_token(&refreshed_token).is_ok());

        // Should have same user data
        let original_claims = jwt_service.validate_token(&original_token).unwrap();
        let refreshed_claims = jwt_service.validate_token(&refreshed_token).unwrap();

        assert_eq!(original_claims.user_id, refreshed_claims.user_id);
        assert_eq!(original_claims.username, refreshed_claims.username);
        assert_eq!(original_claims.email, refreshed_claims.email);
        assert_eq!(original_claims.roles, refreshed_claims.roles);

        // Refreshed token should have later expiration
        assert!(refreshed_claims.exp > original_claims.exp);
    }

    #[test]
    fn test_jwt_different_secrets() {
        let service1 = TestJwtService::new();
        let mut service2 = TestJwtService::new();
        service2.secret = "different-secret-key-for-testing".to_string();

        let user_id = Uuid::new_v4();

        // Generate token with service1
        let token = service1
            .generate_token(user_id, "test", "test@example.com", vec![], Duration::hours(1))
            .unwrap();

        // service1 should validate successfully
        assert!(service1.validate_token(&token).is_ok());

        // service2 should fail validation (different secret)
        assert!(service2.validate_token(&token).is_err());
    }

    #[test]
    fn test_jwt_claims_validation() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        let token = jwt_service
            .generate_token(
                user_id,
                "testuser",
                "test@example.com",
                vec!["admin".to_string(), "user".to_string()],
                Duration::hours(1),
            )
            .unwrap();

        let claims = jwt_service.validate_token(&token).unwrap();

        // Check all claims are present
        assert!(!claims.sub.is_empty());
        assert!(claims.exp > 0);
        assert!(claims.iat > 0);
        assert_eq!(claims.iss, "quake-app");
        assert_eq!(claims.aud, "quake-users");
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.roles.len(), 2);
        assert!(claims.roles.contains(&"admin".to_string()));
        assert!(claims.roles.contains(&"user".to_string()));

        // Expiration should be in the future
        let now = Utc::now().timestamp();
        assert!(claims.exp > now);

        // Issued at should be in the past or now
        assert!(claims.iat <= now);
    }

    #[test]
    fn test_jwt_role_based_tokens() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        let role_scenarios = vec![
            (vec!["user".to_string()], "Basic user"),
            (vec!["admin".to_string()], "Administrator"),
            (vec!["user".to_string(), "moderator".to_string()], "User with moderation"),
            (vec!["admin".to_string(), "super_admin".to_string()], "Super administrator"),
            (vec![], "No roles"),
        ];

        for (roles, description) in role_scenarios {
            let token = jwt_service
                .generate_token(user_id, "test", "test@example.com", roles.clone(), Duration::hours(1))
                .unwrap();

            let claims = jwt_service.validate_token(&token).unwrap();
            assert_eq!(claims.roles, roles, "Failed for scenario: {}", description);
        }
    }

    #[test]
    fn test_jwt_token_tampering() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        let original_token = jwt_service
            .generate_token(user_id, "test", "test@example.com", vec!["user".to_string()], Duration::hours(1))
            .unwrap();

        // Try to tamper with the token by changing characters
        let mut tampered_token = original_token.clone();
        if let Some(pos) = tampered_token.find('.') {
            tampered_token.replace_range(pos+1..pos+2, "X");
        }

        // Tampered token should fail validation
        assert!(jwt_service.validate_token(&tampered_token).is_err());

        // Original should still work
        assert!(jwt_service.validate_token(&original_token).is_ok());
    }

    #[test]
    fn test_jwt_concurrent_operations() {
        use std::sync::Arc;
        use std::thread;

        let jwt_service = Arc::new(TestJwtService::new());
        let mut handles = vec![];

        // Generate tokens concurrently
        for i in 0..5 {
            let service = jwt_service.clone();
            let handle = thread::spawn(move || {
                let user_id = Uuid::new_v4();
                let username = format!("user{}", i);
                let email = format!("user{}@example.com", i);

                service.generate_token(
                    user_id,
                    &username,
                    &email,
                    vec!["user".to_string()],
                    Duration::hours(1),
                )
            });
            handles.push(handle);
        }

        let mut tokens = vec![];
        for handle in handles {
            let token = handle.join().unwrap().unwrap();
            tokens.push(token);
        }

        // All tokens should be unique
        let unique_tokens: std::collections::HashSet<_> = tokens.iter().collect();
        assert_eq!(tokens.len(), unique_tokens.len());

        // All tokens should validate
        for token in &tokens {
            assert!(jwt_service.validate_token(token).is_ok());
        }
    }

    #[test]
    fn test_jwt_edge_cases() {
        let jwt_service = TestJwtService::new();
        let user_id = Uuid::new_v4();

        // Very long username
        let long_username = "a".repeat(1000);
        let token = jwt_service.generate_token(
            user_id,
            &long_username,
            "test@example.com",
            vec![],
            Duration::hours(1),
        );
        assert!(token.is_ok());

        // Unicode in claims
        let unicode_username = "用户名";
        let unicode_email = "测试@example.com";
        let token = jwt_service.generate_token(
            user_id,
            unicode_username,
            unicode_email,
            vec!["用户".to_string()],
            Duration::hours(1),
        );
        assert!(token.is_ok());

        let claims = jwt_service.validate_token(&token.unwrap()).unwrap();
        assert_eq!(claims.username, unicode_username);
        assert_eq!(claims.email, unicode_email);
        assert!(claims.roles.contains(&"用户".to_string()));
    }
}