//! User service implementation with standardized error handling
//!
//! This service handles user-related business logic and returns AppError
//! for consistent error handling across gRPC and HTTP protocols.

use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login, send_command},
    errors::AppError,
    infrastructure::auth::JwtService,
    models::User,
    repositories::UserRepository,
};

#[derive(Clone)]
pub struct UserService {
    pub repo: PostgreSQL,
    pub sender: mpsc::Sender<CommandMessage>,
    pub jwt_service: JwtService,
}

impl UserService {
    pub fn new(repo: PostgreSQL, sender: mpsc::Sender<CommandMessage>) -> Self {
        Self {
            repo,
            sender,
            jwt_service: JwtService::default(),
        }
    }

    /// Handle user creation with proper error conversion
    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(cmd.password.as_bytes(), &salt)
            .map_err(|_| AppError::Internal {
                message: "Password hashing failed".to_string(),
            })?
            .to_string();

        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
            password_hash,
        };

        self.repo.save_user(user).await?;
        Ok(())
    }

    /// Handle user lookup by ID with proper error conversion
    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        self.repo.find_user_by_id(id).await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(id.to_string()),
                },
                _ => AppError::Database {
                    message: format!("Failed to query user: {}", e),
                },
            })
    }

    /// Handle user login with proper error conversion
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
                message: "Invalid password hash format".to_string(),
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
                message: "Failed to generate authentication token".to_string(),
            })
    }

    /// Send user creation command (async, fire-and-forget)
    pub async fn create_user(&self, cmd: CreateUser) {
        send_command(self.sender.clone(), CommandMessage::CreateUser(cmd)).await;
    }

    /// Send login command (async, fire-and-forget)
    pub async fn login(&self, cmd: Login) {
        send_command(self.sender.clone(), CommandMessage::Login(cmd)).await;
    }
}