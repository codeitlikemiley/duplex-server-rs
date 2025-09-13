use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login, RegisterUser, VerifyEmail, send_command},
    errors::AppError,
    infrastructure::auth::JwtService,
    models::{User, UserProfile, UserStatus},
    repositories::{UserRepository, UserProfileRepository},
};

#[derive(Clone)]
pub struct UserApplicationService {
    pub repo: PostgreSQL,
    pub profile_repo: PostgreSQL,
    pub sender: mpsc::Sender<CommandMessage>,
    pub jwt_service: JwtService,
}

impl UserApplicationService {
    pub fn new(repo: PostgreSQL, sender: mpsc::Sender<CommandMessage>) -> Self {
        Self {
            repo: repo.clone(),
            profile_repo: repo,
            sender,
            jwt_service: JwtService::default(),
        }
    }

    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(cmd.password.as_bytes(), &salt)
            .map_err(|_| AppError::Internal {
                message: "Password hashing failed".to_string(),
            })?
            .to_string();

        let now = Utc::now();
        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
            password_hash,
            email_verified: true, // Legacy users are considered verified
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        };

        self.repo.save_user(user).await.map_err(|e| AppError::Database {
            message: format!("Failed to save user: {}", e),
        })?;
        Ok(())
    }

    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        self.repo.find_user_by_id(id).await.map_err(|e| AppError::Database {
            message: format!("Failed to query user: {}", e),
        })
    }

    pub async fn handle_login(&self, cmd: Login) -> Result<String, AppError> {
        // Find user by email
        let user = match self.repo.find_user_by_email(&cmd.email).await? {
            Some(user) => user,
            None => return Err(AppError::Authentication {
                message: "Invalid email or password".to_string(),
            }),
        };

        // Verify password
        let argon2 = Argon2::default();
        let parsed_hash = argon2::password_hash::PasswordHash::new(&user.password_hash)
            .map_err(|_| AppError::Internal {
                message: "Password hash parsing failed".to_string(),
            })?;

        argon2
            .verify_password(cmd.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Authentication {
                message: "Invalid email or password".to_string(),
            })?;

        // Generate JWT token
        self.jwt_service
            .generate_token(user.id, &user.email)
            .map_err(|_| AppError::Internal {
                message: "Token generation failed".to_string(),
            })
    }

    /// Handle user registration with profile creation and email verification
    pub async fn handle_register_user(&self, cmd: RegisterUser) -> Result<(), AppError> {
        let now = Utc::now();

        // Hash password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(cmd.password.as_bytes(), &salt)
            .map_err(|_| AppError::Internal {
                message: "Password hashing failed".to_string(),
            })?
            .to_string();

        // Create user with pending verification status
        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username.clone(),
            email: cmd.email.clone(),
            password_hash,
            email_verified: false,
            status: UserStatus::PendingVerification,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        };

        // Save user to database
        self.repo.save_user(user.clone()).await?;

        // Create user profile
        let profile = UserProfile::new(user.id);
        let profile = UserProfile {
            first_name: cmd.first_name,
            last_name: cmd.last_name,
            ..profile
        };

        // Save profile to database
        self.profile_repo.save_profile(profile).await?;

        // TODO: Send email verification
        // self.send_verification_email(&user).await?;

        tracing::info!("User registered successfully: {} ({})", user.username, user.email);
        Ok(())
    }

    /// Handle email verification
    pub async fn handle_verify_email(&self, cmd: VerifyEmail) -> Result<(), AppError> {
        // TODO: Verify token and update user status
        // This would involve checking the verification token against stored tokens
        // and updating the user's email_verified status to true

        tracing::info!("Email verification attempted for user: {}", cmd.user_id);
        Ok(())
    }

    /// Send login command (async, fire-and-forget)
    pub async fn login(&self, cmd: Login) {
        send_command(self.sender.clone(), CommandMessage::Login(cmd)).await;
    }

    /// Send user registration command (async, fire-and-forget)
    pub async fn register_user(&self, cmd: RegisterUser) {
        send_command(self.sender.clone(), CommandMessage::RegisterUser(cmd)).await;
    }

    /// Send email verification command (async, fire-and-forget)
    pub async fn verify_email(&self, cmd: VerifyEmail) {
        send_command(self.sender.clone(), CommandMessage::VerifyEmail(cmd)).await;
    }
}