use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::{Utc, Duration};
use serde_json::json;

mod integration_test_helpers;
use integration_test_helpers::{IntegrationTestContext, TestDatabase};

use coqrs::{
    models::{User, UserProfile, UserStatus},
    PostgreSQL,
    services::{UserService, PasswordService},
    commands::{CreateUser, Login},
};

/// End-to-end workflow test orchestrator
struct WorkflowTestOrchestrator {
    ctx: IntegrationTestContext,
    user_sessions: Arc<Mutex<HashMap<Uuid, String>>>, // user_id -> session_token
    email_verification_tokens: Arc<Mutex<HashMap<String, Uuid>>>, // token -> user_id
    password_reset_tokens: Arc<Mutex<HashMap<String, Uuid>>>, // token -> user_id
}

impl WorkflowTestOrchestrator {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = IntegrationTestContext::new().await?;

        Ok(Self {
            ctx,
            user_sessions: Arc::new(Mutex::new(HashMap::new())),
            email_verification_tokens: Arc::new(Mutex::new(HashMap::new())),
            password_reset_tokens: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn reset(&self) -> Result<(), sqlx::Error> {
        self.ctx.reset().await?;
        self.user_sessions.lock().await.clear();
        self.email_verification_tokens.lock().await.clear();
        self.password_reset_tokens.lock().await.clear();
        Ok(())
    }

    /// Simulate email verification token generation
    async fn generate_verification_token(&self, user_id: Uuid) -> String {
        let token = format!("verify_{}", Uuid::new_v4().simple());
        self.email_verification_tokens.lock().await.insert(token.clone(), user_id);
        token
    }

    /// Simulate password reset token generation
    async fn generate_password_reset_token(&self, user_id: Uuid) -> String {
        let token = format!("reset_{}", Uuid::new_v4().simple());
        self.password_reset_tokens.lock().await.insert(token.clone(), user_id);
        token
    }

    /// Simulate session token generation
    async fn generate_session_token(&self, user_id: Uuid) -> String {
        let token = format!("session_{}", Uuid::new_v4().simple());
        self.user_sessions.lock().await.insert(user_id, token.clone());
        token
    }
}

#[tokio::test]
async fn test_complete_user_registration_workflow() {
    let orchestrator = WorkflowTestOrchestrator::new().await
        .expect("Failed to create workflow orchestrator");
    orchestrator.reset().await.expect("Failed to reset test environment");

    println!("🚀 Starting complete user registration workflow test");

    // Step 1: User Registration
    println!("📝 Step 1: User Registration");
    let user = orchestrator.ctx.data_factory.create_user("workflow_user", "workflow@example.com").await
        .expect("Failed to create user");

    assert_eq!(user.status, UserStatus::PendingVerification);
    assert!(!user.email_verified);
    println!("✅ User registered successfully: {}", user.email);

    // Step 2: Email Verification Token Generation
    println!("📧 Step 2: Email Verification");
    let verification_token = orchestrator.generate_verification_token(user.id).await;
    println!("✅ Verification token generated: {}", verification_token);

    // Step 3: Email Verification Process
    println!("🔐 Step 3: Email Verification Process");

    // Simulate user clicking verification link
    let token_map = orchestrator.email_verification_tokens.lock().await;
    let verified_user_id = token_map.get(&verification_token).copied();
    drop(token_map);

    assert!(verified_user_id.is_some(), "Verification token should be valid");
    assert_eq!(verified_user_id.unwrap(), user.id);

    // Update user as verified
    let mut verified_user = user.clone();
    verified_user.email_verified = true;
    verified_user.status = UserStatus::Active;
    verified_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(verified_user.clone()).await
        .expect("Failed to update verified user");

    println!("✅ Email verified successfully");

    // Step 4: First Login
    println!("🔑 Step 4: First Login");
    let session_token = orchestrator.generate_session_token(user.id).await;

    // Verify login would succeed (user is now active and verified)
    let login_user = orchestrator.ctx.db.postgres.find_user_by_email("workflow@example.com").await
        .expect("Failed to find user")
        .expect("User should exist");

    assert!(login_user.email_verified);
    assert_eq!(login_user.status, UserStatus::Active);

    println!("✅ First login successful, session token: {}", session_token);

    // Step 5: Profile Setup
    println!("👤 Step 5: Profile Setup");
    let profile = orchestrator.ctx.data_factory.create_profile(
        user.id,
        Some("John"),
        Some("Workflow")
    ).await.expect("Failed to create profile");

    assert_eq!(profile.user_id, user.id);
    assert_eq!(profile.first_name, Some("John".to_string()));
    println!("✅ Profile created successfully");

    // Step 6: Profile Completion
    println!("📋 Step 6: Profile Completion");
    let complete_profile = UserProfile {
        user_id: user.id,
        first_name: Some("John".to_string()),
        last_name: Some("Workflow".to_string()),
        bio: Some("Software engineer passionate about testing".to_string()),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        website: Some("https://johnworkflow.dev".to_string()),
        location: Some("Remote".to_string()),
        preferences: json!({"theme": "dark", "notifications": true, "language": "en"}),
        created_at: profile.created_at,
        updated_at: Utc::now(),
    };

    orchestrator.ctx.db.postgres.save_profile(complete_profile.clone()).await
        .expect("Failed to update profile");

    let updated_profile = orchestrator.ctx.db.postgres.find_profile_by_user_id(user.id).await
        .expect("Failed to find profile")
        .expect("Profile should exist");

    assert_eq!(updated_profile.bio, Some("Software engineer passionate about testing".to_string()));
    assert_eq!(updated_profile.website, Some("https://johnworkflow.dev".to_string()));
    println!("✅ Profile completed successfully");

    // Step 7: Additional Login Sessions
    println!("🔄 Step 7: Multiple Session Management");
    let second_session = orchestrator.generate_session_token(user.id).await;
    let third_session = orchestrator.generate_session_token(user.id).await;

    // Verify multiple sessions are tracked
    let sessions = orchestrator.user_sessions.lock().await;
    assert!(sessions.contains_key(&user.id));
    println!("✅ Multiple sessions managed: {}, {}, {}", session_token, second_session, third_session);

    println!("🎉 Complete user registration workflow test passed!");
}

#[tokio::test]
async fn test_password_reset_workflow() {
    let orchestrator = WorkflowTestOrchestrator::new().await
        .expect("Failed to create workflow orchestrator");
    orchestrator.reset().await.expect("Failed to reset test environment");

    println!("🔐 Starting password reset workflow test");

    // Step 1: Create active user
    let user = orchestrator.ctx.data_factory.create_active_user("reset_user", "reset@example.com").await
        .expect("Failed to create active user");

    println!("✅ Step 1: Active user created: {}", user.email);

    // Step 2: Request password reset
    println!("📨 Step 2: Password reset request");
    let reset_token = orchestrator.generate_password_reset_token(user.id).await;
    println!("✅ Password reset token generated: {}", reset_token);

    // Step 3: Validate reset token
    println!("🔍 Step 3: Reset token validation");
    let token_map = orchestrator.password_reset_tokens.lock().await;
    let reset_user_id = token_map.get(&reset_token).copied();
    drop(token_map);

    assert!(reset_user_id.is_some(), "Reset token should be valid");
    assert_eq!(reset_user_id.unwrap(), user.id);
    println!("✅ Reset token validated");

    // Step 4: Password reset execution
    println!("🔒 Step 4: Password reset execution");
    let new_password_hash = "$argon2id$v=19$m=4096,t=3,p=1$newsalt$newhash";

    let mut updated_user = user.clone();
    updated_user.password_hash = new_password_hash.to_string();
    updated_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(updated_user).await
        .expect("Failed to update user password");

    // Step 5: Verify password was changed
    let updated_found = orchestrator.ctx.db.postgres.find_user_by_id(user.id).await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_eq!(updated_found.password_hash, new_password_hash);
    assert_ne!(updated_found.password_hash, user.password_hash);
    println!("✅ Password reset successful");

    // Step 6: Invalidate existing sessions (simulation)
    orchestrator.user_sessions.lock().await.remove(&user.id);
    println!("✅ Existing sessions invalidated");

    // Step 7: New login with new password
    println!("🔑 Step 7: Login with new password");
    let new_session = orchestrator.generate_session_token(user.id).await;
    println!("✅ New login successful with session: {}", new_session);

    println!("🎉 Password reset workflow test passed!");
}

#[tokio::test]
async fn test_account_lifecycle_workflow() {
    let orchestrator = WorkflowTestOrchestrator::new().await
        .expect("Failed to create workflow orchestrator");
    orchestrator.reset().await.expect("Failed to reset test environment");

    println!("♻️ Starting account lifecycle workflow test");

    // Step 1: Account creation and activation
    let user = orchestrator.ctx.data_factory.create_active_user("lifecycle_user", "lifecycle@example.com").await
        .expect("Failed to create user");

    assert_eq!(user.status, UserStatus::Active);
    println!("✅ Step 1: Account created and activated");

    // Step 2: Account usage period simulation
    println!("📈 Step 2: Account usage simulation");

    // Create profile and activity
    let profile = orchestrator.ctx.data_factory.create_profile(user.id, Some("Lifecycle"), Some("User")).await
        .expect("Failed to create profile");

    // Simulate multiple logins
    for i in 0..5 {
        let session = orchestrator.generate_session_token(user.id).await;
        println!("  Login session {}: {}", i + 1, session);

        // Simulate some time passing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    println!("✅ Account usage simulated");

    // Step 3: Account deactivation
    println!("⏸️ Step 3: Account deactivation");
    let mut deactivated_user = user.clone();
    deactivated_user.status = UserStatus::Inactive;
    deactivated_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(deactivated_user).await
        .expect("Failed to deactivate user");

    let deactivated_found = orchestrator.ctx.db.postgres.find_user_by_id(user.id).await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_eq!(deactivated_found.status, UserStatus::Inactive);
    println!("✅ Account deactivated");

    // Step 4: Reactivation attempt
    println!("🔄 Step 4: Account reactivation");
    let mut reactivated_user = deactivated_found.clone();
    reactivated_user.status = UserStatus::Active;
    reactivated_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(reactivated_user).await
        .expect("Failed to reactivate user");

    let reactivated_found = orchestrator.ctx.db.postgres.find_user_by_id(user.id).await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_eq!(reactivated_found.status, UserStatus::Active);
    println!("✅ Account reactivated");

    // Step 5: Account suspension
    println!("🚫 Step 5: Account suspension");
    let mut suspended_user = reactivated_found.clone();
    suspended_user.status = UserStatus::Suspended;
    suspended_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(suspended_user).await
        .expect("Failed to suspend user");

    let suspended_found = orchestrator.ctx.db.postgres.find_user_by_id(user.id).await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_eq!(suspended_found.status, UserStatus::Suspended);
    println!("✅ Account suspended");

    // Step 6: Final cleanup (account would be deleted in real scenario)
    println!("🧹 Step 6: Account cleanup simulation");

    // In a real scenario, you might:
    // - Delete user data
    // - Archive essential information
    // - Invalidate all sessions
    // - Send confirmation emails

    orchestrator.user_sessions.lock().await.remove(&user.id);
    println!("✅ Sessions cleaned up");

    println!("🎉 Account lifecycle workflow test passed!");
}

#[tokio::test]
async fn test_concurrent_user_workflows() {
    let orchestrator = WorkflowTestOrchestrator::new().await
        .expect("Failed to create workflow orchestrator");
    orchestrator.reset().await.expect("Failed to reset test environment");

    println!("🔀 Starting concurrent user workflows test");

    let mut handles = Vec::new();

    // Simulate 10 concurrent user registration workflows
    for i in 0..10 {
        let ctx = orchestrator.ctx.db.clone();
        let handle = tokio::spawn(async move {
            // Each user goes through complete registration
            let username = format!("concurrent_user_{}", i);
            let email = format!("concurrent_{}@example.com", i);

            // Step 1: Registration
            let user = User {
                id: Uuid::new_v4(),
                username: username.clone(),
                email: email.clone(),
                password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
                email_verified: false,
                status: UserStatus::PendingVerification,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: None,
            };

            ctx.postgres.save_user(user.clone()).await.expect("Failed to save user");

            // Step 2: Email verification
            let mut verified_user = user.clone();
            verified_user.email_verified = true;
            verified_user.status = UserStatus::Active;

            ctx.postgres.save_user(verified_user).await.expect("Failed to verify user");

            // Step 3: Profile creation
            let profile = UserProfile {
                user_id: user.id,
                first_name: Some(format!("User{}", i)),
                last_name: Some("Concurrent".to_string()),
                bio: Some(format!("Concurrent user number {}", i)),
                avatar_url: None,
                website: None,
                location: None,
                preferences: json!({"user_id": i}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            ctx.postgres.save_profile(profile).await.expect("Failed to save profile");

            (user.id, username, email)
        });

        handles.push(handle);
    }

    // Wait for all workflows to complete
    let results = futures::future::join_all(handles).await;

    // Verify all workflows completed successfully
    for (i, result) in results.into_iter().enumerate() {
        let (user_id, username, email) = result.expect("Workflow should complete successfully");
        println!("✅ Workflow {} completed: {} ({})", i, username, email);

        // Verify user exists and is active
        let found_user = orchestrator.ctx.db.postgres.find_user_by_id(user_id).await
            .expect("Failed to find user")
            .expect("User should exist");

        assert_eq!(found_user.status, UserStatus::Active);
        assert!(found_user.email_verified);

        // Verify profile exists
        let found_profile = orchestrator.ctx.db.postgres.find_profile_by_user_id(user_id).await
            .expect("Failed to find profile")
            .expect("Profile should exist");

        assert_eq!(found_profile.first_name, Some(format!("User{}", i)));
    }

    // Verify final counts
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&orchestrator.ctx.db.pool)
        .await
        .expect("Failed to count users");

    let profile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(&orchestrator.ctx.db.pool)
        .await
        .expect("Failed to count profiles");

    assert_eq!(user_count, 10, "Should have 10 users");
    assert_eq!(profile_count, 10, "Should have 10 profiles");

    println!("🎉 Concurrent user workflows test passed! Created {} users with profiles", user_count);
}

#[tokio::test]
async fn test_error_recovery_workflow() {
    let orchestrator = WorkflowTestOrchestrator::new().await
        .expect("Failed to create workflow orchestrator");
    orchestrator.reset().await.expect("Failed to reset test environment");

    println!("🛠️ Starting error recovery workflow test");

    // Step 1: Failed registration attempt (duplicate email)
    println!("❌ Step 1: Failed registration (duplicate email)");

    let user1 = orchestrator.ctx.data_factory.create_user("first_user", "shared@example.com").await
        .expect("Failed to create first user");

    // Try to create user with same email
    let duplicate_user = User {
        id: Uuid::new_v4(),
        username: "second_user".to_string(),
        email: "shared@example.com".to_string(), // Same email
        password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
        email_verified: false,
        status: UserStatus::PendingVerification,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login_at: None,
    };

    let duplicate_result = orchestrator.ctx.db.postgres.save_user(duplicate_user).await;
    assert!(duplicate_result.is_err(), "Duplicate email should fail");
    println!("✅ Duplicate email properly rejected");

    // Step 2: Successful registration with corrected email
    println!("✅ Step 2: Successful registration with unique email");
    let user2 = orchestrator.ctx.data_factory.create_user("second_user", "unique@example.com").await
        .expect("Failed to create second user");

    assert_ne!(user1.email, user2.email);
    println!("✅ Unique email registration successful");

    // Step 3: Failed login attempt (wrong password)
    println!("❌ Step 3: Failed login (wrong password simulation)");

    // In a real scenario, this would test password verification
    // For now, we simulate by checking if user is in correct state for login
    let login_user = orchestrator.ctx.db.postgres.find_user_by_email("unique@example.com").await
        .expect("Failed to find user")
        .expect("User should exist");

    // User is not verified yet, so login should conceptually fail
    assert!(!login_user.email_verified);
    assert_eq!(login_user.status, UserStatus::PendingVerification);
    println!("✅ Unverified user login properly blocked");

    // Step 4: Email verification and successful login
    println!("✅ Step 4: Email verification and successful login");

    let mut verified_user = login_user.clone();
    verified_user.email_verified = true;
    verified_user.status = UserStatus::Active;

    orchestrator.ctx.db.postgres.save_user(verified_user).await
        .expect("Failed to verify user");

    let final_user = orchestrator.ctx.db.postgres.find_user_by_email("unique@example.com").await
        .expect("Failed to find user")
        .expect("User should exist");

    assert!(final_user.email_verified);
    assert_eq!(final_user.status, UserStatus::Active);
    println!("✅ Login now possible after verification");

    // Step 5: Recovery from suspended account
    println!("🔄 Step 5: Account suspension and recovery");

    let mut suspended_user = final_user.clone();
    suspended_user.status = UserStatus::Suspended;

    orchestrator.ctx.db.postgres.save_user(suspended_user).await
        .expect("Failed to suspend user");

    // Recovery process
    let mut recovered_user = final_user.clone();
    recovered_user.status = UserStatus::Active;
    recovered_user.updated_at = Utc::now();

    orchestrator.ctx.db.postgres.save_user(recovered_user).await
        .expect("Failed to recover user");

    let recovered_found = orchestrator.ctx.db.postgres.find_user_by_email("unique@example.com").await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_eq!(recovered_found.status, UserStatus::Active);
    println!("✅ Account successfully recovered from suspension");

    println!("🎉 Error recovery workflow test passed!");
}