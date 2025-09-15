use chrono::Utc;
use uuid::Uuid;
use sqlx::{Pool, Postgres};

use crate::{
    errors::AppError,
    models::{User, UserProfile},
    infrastructure::repositories::PostgreSQL,
};

pub struct ProfileService {
    db: Pool<Postgres>,
    postgres: PostgreSQL,
}

impl ProfileService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        let postgres = PostgreSQL::new(pool.clone());
        Self { db: pool, postgres }
    }

    /// Get user profile by user ID
    pub async fn get_profile(&self, user_id: Uuid) -> Result<Option<UserProfile>, AppError> {
        let profile = sqlx::query_as!(
            UserProfile,
            r#"
            SELECT user_id, first_name, last_name, bio, avatar_url, website, location, preferences, created_at, updated_at
            FROM user_profiles
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch profile: {}", e)
        })?;

        Ok(profile)
    }

    /// Update user profile
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        updates: UpdateProfileRequest,
    ) -> Result<UserProfile, AppError> {
        // First, check if the user exists
        let user = self.postgres.find_user_by_id(user_id).await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find user: {}", e)
            })?;

        if user.is_none() {
            return Err(AppError::NotFound {
                resource: "User".to_string(),
                id: Some(user_id.to_string()),
            });
        }

        // Check if profile exists, create if not
        let existing_profile = self.get_profile(user_id).await?;

        let profile = if existing_profile.is_some() {
            // Update existing profile
            sqlx::query_as!(
                UserProfile,
                r#"
                UPDATE user_profiles
                SET first_name = COALESCE($2, first_name),
                    last_name = COALESCE($3, last_name),
                    bio = COALESCE($4, bio),
                    avatar_url = COALESCE($5, avatar_url),
                    website = COALESCE($6, website),
                    location = COALESCE($7, location),
                    preferences = COALESCE($8, preferences),
                    updated_at = $9
                WHERE user_id = $1
                RETURNING user_id, first_name, last_name, bio, avatar_url, website, location, preferences, created_at, updated_at
                "#,
                user_id,
                updates.first_name,
                updates.last_name,
                updates.bio,
                updates.avatar_url,
                updates.website,
                updates.location,
                updates.preferences,
                Utc::now()
            )
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to update profile: {}", e)
            })?
        } else {
            // Create new profile
            sqlx::query_as!(
                UserProfile,
                r#"
                INSERT INTO user_profiles (user_id, first_name, last_name, bio, avatar_url, website, location, preferences, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING user_id, first_name, last_name, bio, avatar_url, website, location, preferences, created_at, updated_at
                "#,
                user_id,
                updates.first_name,
                updates.last_name,
                updates.bio,
                updates.avatar_url,
                updates.website,
                updates.location,
                updates.preferences.clone().unwrap_or(serde_json::json!({})),
                Utc::now(),
                Utc::now()
            )
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to create profile: {}", e)
            })?
        };

        tracing::info!("Profile updated for user: {}", user_id);
        Ok(profile)
    }

    /// Update user account details (username, email)
    pub async fn update_account(
        &self,
        user_id: Uuid,
        updates: UpdateAccountRequest,
    ) -> Result<User, AppError> {
        // Validate username if provided
        if let Some(ref username) = updates.username {
            self.validate_username(username)?;

            // Check if username is already taken
            if let Some(existing_user) = self.postgres.find_user_by_email(username).await
                .map_err(|e| AppError::Database {
                    message: format!("Failed to check username: {}", e)
                })? {
                if existing_user.id != user_id {
                    return Err(AppError::Validation {
                        field: "username".to_string(),
                        message: "Username already taken".to_string(),
                    });
                }
            }
        }

        // Validate email if provided
        if let Some(ref email) = updates.email {
            self.validate_email(email)?;

            // Check if email is already taken
            if let Some(existing_user) = self.postgres.find_user_by_email(email).await
                .map_err(|e| AppError::Database {
                    message: format!("Failed to check email: {}", e)
                })? {
                if existing_user.id != user_id {
                    return Err(AppError::Validation {
                        field: "email".to_string(),
                        message: "Email already in use".to_string(),
                    });
                }
            }
        }

        // Update user account
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = COALESCE($2, username),
                email = COALESCE($3, email),
                email_verified = CASE
                    WHEN $3 IS NOT NULL AND email != $3 THEN false
                    ELSE email_verified
                END,
                updated_at = $4
            WHERE id = $1
            RETURNING id, username, email, password_hash, email_verified,
                      status as "status: crate::models::UserStatus",
                      created_at, updated_at, last_login_at
            "#,
            user_id,
            updates.username,
            updates.email,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update account: {}", e)
        })?;

        // If email was changed, we should send a verification email
        if updates.email.is_some() && !user.email_verified {
            // TODO: Trigger email verification
            tracing::info!("Email changed for user {}, verification required", user_id);
        }

        tracing::info!("Account updated for user: {}", user_id);
        Ok(user)
    }

    /// Delete user profile (soft delete by clearing data)
    pub async fn delete_profile(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE user_profiles
            SET first_name = NULL,
                last_name = NULL,
                bio = NULL,
                avatar_url = NULL,
                website = NULL,
                location = NULL,
                updated_at = $2
            WHERE user_id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete profile: {}", e)
        })?;

        tracing::info!("Profile deleted for user: {}", user_id);
        Ok(())
    }

    /// Validate username format
    fn validate_username(&self, username: &str) -> Result<(), AppError> {
        if username.len() < 3 {
            return Err(AppError::Validation {
                field: "username".to_string(),
                message: "Username must be at least 3 characters".to_string(),
            });
        }

        if username.len() > 30 {
            return Err(AppError::Validation {
                field: "username".to_string(),
                message: "Username must be less than 30 characters".to_string(),
            });
        }

        // Must start with a letter and contain only letters, numbers, and underscores
        let re = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
        if !re.is_match(username) {
            return Err(AppError::Validation {
                field: "username".to_string(),
                message: "Username must start with a letter and contain only letters, numbers, and underscores".to_string(),
            });
        }

        Ok(())
    }

    /// Validate email format
    fn validate_email(&self, email: &str) -> Result<(), AppError> {
        let re = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if !re.is_match(email) {
            return Err(AppError::Validation {
                field: "email".to_string(),
                message: "Invalid email format".to_string(),
            });
        }

        Ok(())
    }
}

// Request DTOs
#[derive(Debug, serde::Deserialize)]
pub struct UpdateProfileRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
    pub preferences: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateAccountRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}