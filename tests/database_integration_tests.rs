use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;

mod integration_test_helpers;
use integration_test_helpers::{IntegrationTestContext, TestDatabase, TestDataFactory};

use coqrs::{
    models::{User, UserProfile, UserStatus},
    PostgreSQL,
};

/// Test PostgreSQL repository implementations with real database
#[tokio::test]
async fn test_user_repository_crud_operations() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let postgres = PostgreSQL::new(test_db.pool.clone());

    // Test user creation
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: false,
        status: UserStatus::PendingVerification,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    // Test save_user
    let save_result = postgres.save_user(user.clone()).await;
    assert!(save_result.is_ok(), "Failed to save user: {:?}", save_result.err());

    // Test find_user_by_id
    let found_user = postgres.find_user_by_id(user_id).await;
    assert!(found_user.is_ok(), "Failed to find user by ID: {:?}", found_user.err());

    let found_user = found_user.unwrap();
    assert!(found_user.is_some(), "User not found by ID");

    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, user_id);
    assert_eq!(found_user.username, "testuser");
    assert_eq!(found_user.email, "test@example.com");
    assert!(!found_user.email_verified);

    // Test find_user_by_email
    let found_by_email = postgres.find_user_by_email("test@example.com").await;
    assert!(found_by_email.is_ok(), "Failed to find user by email: {:?}", found_by_email.err());

    let found_by_email = found_by_email.unwrap();
    assert!(found_by_email.is_some(), "User not found by email");
    assert_eq!(found_by_email.unwrap().id, user_id);

    // Test user update
    let mut updated_user = user.clone();
    updated_user.email_verified = true;
    updated_user.status = UserStatus::Active;
    updated_user.last_login_at = Some(Utc::now());

    let update_result = postgres.save_user(updated_user.clone()).await;
    assert!(update_result.is_ok(), "Failed to update user: {:?}", update_result.err());

    // Verify update
    let updated_found = postgres.find_user_by_id(user_id).await.unwrap().unwrap();
    assert!(updated_found.email_verified);
    assert_eq!(updated_found.status, UserStatus::Active);
    assert!(updated_found.last_login_at.is_some());

    // Test non-existent user
    let non_existent = postgres.find_user_by_id(Uuid::new_v4()).await;
    assert!(non_existent.is_ok());
    assert!(non_existent.unwrap().is_none());

    // Cleanup
    test_db.drop_database().await.expect("Failed to drop test database");
}

#[tokio::test]
async fn test_user_profile_repository_crud_operations() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let postgres = PostgreSQL::new(test_db.pool.clone());

    // Create a user first
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: true,
        status: UserStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    postgres.save_user(user).await.expect("Failed to save user");

    // Test profile creation
    let profile = UserProfile {
        user_id,
        first_name: Some("John".to_string()),
        last_name: Some("Doe".to_string()),
        bio: Some("Software developer".to_string()),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        website: Some("https://johndoe.dev".to_string()),
        location: Some("San Francisco, CA".to_string()),
        preferences: serde_json::json!({"theme": "dark", "notifications": true}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Test save_profile
    let save_result = postgres.save_profile(profile.clone()).await;
    assert!(save_result.is_ok(), "Failed to save profile: {:?}", save_result.err());

    // Test find_profile_by_user_id
    let found_profile = postgres.find_profile_by_user_id(user_id).await;
    assert!(found_profile.is_ok(), "Failed to find profile: {:?}", found_profile.err());

    let found_profile = found_profile.unwrap();
    assert!(found_profile.is_some(), "Profile not found");

    let found_profile = found_profile.unwrap();
    assert_eq!(found_profile.user_id, user_id);
    assert_eq!(found_profile.first_name, Some("John".to_string()));
    assert_eq!(found_profile.last_name, Some("Doe".to_string()));
    assert_eq!(found_profile.bio, Some("Software developer".to_string()));

    // Test profile update
    let mut updated_profile = profile.clone();
    updated_profile.first_name = Some("Jane".to_string());
    updated_profile.bio = Some("Senior software engineer".to_string());
    updated_profile.preferences = serde_json::json!({"theme": "light", "notifications": false});

    let update_result = postgres.save_profile(updated_profile.clone()).await;
    assert!(update_result.is_ok(), "Failed to update profile: {:?}", update_result.err());

    // Verify update
    let updated_found = postgres.find_profile_by_user_id(user_id).await.unwrap().unwrap();
    assert_eq!(updated_found.first_name, Some("Jane".to_string()));
    assert_eq!(updated_found.bio, Some("Senior software engineer".to_string()));
    assert_eq!(updated_found.preferences["theme"], "light");

    // Test non-existent profile
    let non_existent = postgres.find_profile_by_user_id(Uuid::new_v4()).await;
    assert!(non_existent.is_ok());
    assert!(non_existent.unwrap().is_none());

    // Cleanup
    test_db.drop_database().await.expect("Failed to drop test database");
}

#[tokio::test]
async fn test_database_constraints_and_validation() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let postgres = PostgreSQL::new(test_db.pool.clone());

    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: false,
        status: UserStatus::PendingVerification,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    // Save the first user
    postgres.save_user(user.clone()).await.expect("Failed to save first user");

    // Test unique constraint violation for email
    let duplicate_email_user = User {
        id: Uuid::new_v4(),
        username: "differentuser".to_string(),
        email: "test@example.com".to_string(), // Same email
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: false,
        status: UserStatus::PendingVerification,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    let duplicate_result = postgres.save_user(duplicate_email_user).await;
    assert!(duplicate_result.is_err(), "Should fail due to unique email constraint");

    // Test unique constraint violation for username
    let duplicate_username_user = User {
        id: Uuid::new_v4(),
        username: "testuser".to_string(), // Same username
        email: "different@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: false,
        status: UserStatus::PendingVerification,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    let duplicate_username_result = postgres.save_user(duplicate_username_user).await;
    assert!(duplicate_username_result.is_err(), "Should fail due to unique username constraint");

    // Cleanup
    test_db.drop_database().await.expect("Failed to drop test database");
}

#[tokio::test]
async fn test_database_transactions() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");

    // Test successful transaction
    let mut tx = test_db.pool.begin().await.expect("Failed to start transaction");

    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: "txuser".to_string(),
        email: "tx@example.com".to_string(),
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: true,
        status: UserStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    // Insert user within transaction
    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, email_verified, status, created_at, updated_at, last_login_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    )
    .bind(user.id)
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(user.email_verified)
    .bind(user.status)
    .bind(user.created_at)
    .bind(user.updated_at)
    .bind(user.last_login_at)
    .execute(&mut *tx)
    .await
    .expect("Failed to insert user in transaction");

    // Insert profile within same transaction
    sqlx::query(
        r#"
        INSERT INTO user_profiles (user_id, first_name, last_name, bio, avatar_url, website, location, preferences, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(user_id)
    .bind("Transaction")
    .bind("User")
    .bind("Test user")
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(serde_json::json!({}))
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .expect("Failed to insert profile in transaction");

    // Commit transaction
    tx.commit().await.expect("Failed to commit transaction");

    // Verify both records exist
    let postgres = PostgreSQL::new(test_db.pool.clone());
    let found_user = postgres.find_user_by_id(user_id).await.unwrap();
    assert!(found_user.is_some());

    let found_profile = postgres.find_profile_by_user_id(user_id).await.unwrap();
    assert!(found_profile.is_some());

    // Test transaction rollback
    let mut tx = test_db.pool.begin().await.expect("Failed to start transaction");

    let user_id_2 = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, email_verified, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(user_id_2)
    .bind("rollbackuser")
    .bind("rollback@example.com")
    .bind("$argon2id$v=19$m=4096,t=3,p=1$salt$hash")
    .bind(false)
    .bind(UserStatus::PendingVerification)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .expect("Failed to insert user for rollback test");

    // Rollback transaction
    tx.rollback().await.expect("Failed to rollback transaction");

    // Verify record does not exist
    let rollback_user = postgres.find_user_by_id(user_id_2).await.unwrap();
    assert!(rollback_user.is_none(), "User should not exist after rollback");

    // Cleanup
    test_db.drop_database().await.expect("Failed to drop test database");
}

#[tokio::test]
async fn test_database_performance_bulk_operations() {
    let test_db = TestDatabase::new().await.expect("Failed to create test database");
    let postgres = PostgreSQL::new(test_db.pool.clone());

    let start_time = std::time::Instant::now();

    // Create 100 users
    for i in 0..100 {
        let user = User {
            id: Uuid::new_v4(),
            username: format!("bulkuser{}", i),
            email: format!("bulk{}@example.com", i),
            password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
            email_verified: i % 2 == 0, // Alternate verified status
            status: if i % 3 == 0 { UserStatus::Active } else { UserStatus::PendingVerification },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: if i % 4 == 0 { Some(Utc::now()) } else { None },
        };

        postgres.save_user(user).await.expect("Failed to save bulk user");
    }

    let duration = start_time.elapsed();
    println!("Created 100 users in: {:?}", duration);

    // Verify count
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&test_db.pool)
        .await
        .expect("Failed to count users");

    assert_eq!(count, 100, "Expected 100 users in database");

    // Test query performance
    let start_time = std::time::Instant::now();

    // Find all verified users
    let verified_users: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, email FROM users WHERE email_verified = true"
    )
    .fetch_all(&test_db.pool)
    .await
    .expect("Failed to query verified users");

    let query_duration = start_time.elapsed();
    println!("Queried verified users in: {:?}", query_duration);

    assert_eq!(verified_users.len(), 50, "Expected 50 verified users");

    // Cleanup
    test_db.drop_database().await.expect("Failed to drop test database");
}

#[tokio::test]
async fn test_integration_test_helpers() {
    let ctx = IntegrationTestContext::new().await.expect("Failed to create integration test context");

    // Test data factory
    let user = ctx.data_factory.create_user("factoryuser", "factory@example.com").await
        .expect("Failed to create user with factory");

    assert_eq!(user.username, "factoryuser");
    assert_eq!(user.email, "factory@example.com");

    // Verify user exists in database
    let found_user = ctx.db.postgres.find_user_by_id(user.id).await.unwrap();
    assert!(found_user.is_some());

    // Test profile creation
    let profile = ctx.data_factory.create_profile(user.id, Some("Factory"), Some("User")).await
        .expect("Failed to create profile with factory");

    assert_eq!(profile.user_id, user.id);
    assert_eq!(profile.first_name, Some("Factory".to_string()));

    // Test bulk creation
    let users = ctx.data_factory.create_users(5).await
        .expect("Failed to create bulk users");

    assert_eq!(users.len(), 5);

    // Test cleanup
    ctx.reset().await.expect("Failed to reset test context");

    // Verify cleanup worked
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&ctx.db.pool)
        .await
        .expect("Failed to count users after cleanup");

    assert_eq!(count, 0, "Expected 0 users after cleanup");
}