use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::models::{User, UserProfile};

#[derive(Clone)]
pub struct PostgreSQL {
    pub db: Pool<Postgres>,
}

impl PostgreSQL {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { db: pool }
    }

    pub async fn save_user(&self, user: User) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, email_verified, status, created_at, updated_at, last_login_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                username = EXCLUDED.username,
                email = EXCLUDED.email,
                password_hash = EXCLUDED.password_hash,
                email_verified = EXCLUDED.email_verified,
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at,
                last_login_at = EXCLUDED.last_login_at
            "#,
            user.id,
            user.username,
            user.email,
            user.password_hash,
            user.email_verified,
            user.status as i32,
            user.created_at,
            user.updated_at,
            user.last_login_at
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, email_verified, status as "status: crate::models::UserStatus",
                   created_at, updated_at, last_login_at
            FROM users WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, password_hash, email_verified, status as "status: crate::models::UserStatus",
                   created_at, updated_at, last_login_at
            FROM users WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn save_profile(&self, profile: UserProfile) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO user_profiles (user_id, first_name, last_name, bio, avatar_url, website, location, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id) DO UPDATE SET
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                bio = EXCLUDED.bio,
                avatar_url = EXCLUDED.avatar_url,
                website = EXCLUDED.website,
                location = EXCLUDED.location,
                updated_at = EXCLUDED.updated_at
            "#,
            profile.user_id,
            profile.first_name,
            profile.last_name,
            profile.bio,
            profile.avatar_url,
            profile.website,
            profile.location,
            profile.created_at,
            profile.updated_at
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
