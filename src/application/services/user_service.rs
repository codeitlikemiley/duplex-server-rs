use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login, RegisterUser, VerifyEmail, send_command},
    errors::AppError,
    infrastructure::{auth::JwtService, email::EmailServiceFactory},
    models::{User, UserProfile, UserStatus},
};

#[derive(Clone)]
pub struct UserService {
    pub repo: PostgreSQL,
    pub profile_repo: PostgreSQL,
    pub sender: mpsc::Sender<CommandMessage>,
    pub jwt_service: JwtService,
}

impl UserService {
    pub fn new(repo: PostgreSQL, sender: mpsc::Sender<CommandMessage>) -> Self {
        Self {
            repo: repo.clone(),
            profile_repo: repo,
            sender,
            jwt_service: JwtService::default(),
        }
    }

    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), AppError> {
        // Hash password using the password service
        let password_hash = super::PasswordService::hash_password(&cmd.password)?;

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

        self.repo.save_user(user).await?;
        Ok(())
    }

    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        self.repo.find_user_by_id(id).await
    }

    pub async fn handle_login(&self, cmd: Login) -> Result<String, AppError> {
        // Find user by email
        let user = self
            .repo
            .find_user_by_email(&cmd.email)
            .await?
            .ok_or_else(|| AppError::Authentication {
                message: "Invalid email or password".to_string(),
            })?;

        // Check if email is verified
        if !user.email_verified {
            return Err(AppError::Authentication {
                message: "Please verify your email before logging in".to_string(),
            });
        }

        // Check if account is active
        if user.status != UserStatus::Active {
            return Err(AppError::Authentication {
                message: "Account is not active".to_string(),
            });
        }

        // Verify password
        super::PasswordService::verify_password(&cmd.password, &user.password_hash)?;

        // Create session and return token
        let session_service = super::SessionService::new(self.repo.clone());
        let token = session_service
            .create_session(user.id, &user.email, None, None)
            .await?;

        Ok(token)
    }

    /// Handle user registration with profile creation and email verification
    pub async fn handle_register_user(&self, cmd: RegisterUser) -> Result<(), AppError> {
        let now = Utc::now();

        // Validate password strength
        super::PasswordService::validate_password_strength(&cmd.password)?;

        // Hash password
        let password_hash = super::PasswordService::hash_password(&cmd.password)?;

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

        // Assign default "user" role to the new user
        let rbac_service = super::RbacService::new(self.repo.db.clone());
        rbac_service.assign_role(
            user.id,
            "user",  // Default role for new users
            user.id, // Self-assigned during registration
            None,    // No expiration for default role
        ).await?;

        // Send verification email
        let email_service_impl = EmailServiceFactory::from_env()
            .unwrap_or_else(|_| EmailServiceFactory::console());
        let email_service = super::EmailVerificationService::new(
            self.repo.clone(),
            email_service_impl,
        );

        email_service
            .send_verification_email(user.id, &user.email, &user.username)
            .await?;

        tracing::info!(
            "User registered successfully: {} ({})",
            user.username,
            user.email
        );
        Ok(())
    }

    /// Handle email verification
    pub async fn handle_verify_email(&self, cmd: VerifyEmail) -> Result<(), AppError> {
        let email_service_impl = EmailServiceFactory::from_env()
            .unwrap_or_else(|_| EmailServiceFactory::console());
        let email_service = super::EmailVerificationService::new(
            self.repo.clone(),
            email_service_impl,
        );
        email_service.verify_email_token(cmd.user_id, &cmd.verification_token).await?;

        tracing::info!("Email verified successfully for user: {}", cmd.user_id);
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
