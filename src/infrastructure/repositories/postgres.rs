use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{events::UserCreated, models, repositories::{UserRepository, UserProfileRepository}};

#[derive(Clone, Debug)]
pub struct PostgreSQL {
    db: Pool<Postgres>,
}

impl PostgreSQL {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for PostgreSQL {
    async fn save_user(&self, user: models::User) -> Result<(), sqlx::Error> {
        let result = sqlx::query!(
            "INSERT INTO users (id,username,email,password_hash,email_verified,status,created_at,updated_at,last_login_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            user.id,
            &user.username,
            &user.email,
            &user.password_hash,
            user.email_verified,
            user.status as models::UserStatus,
            user.created_at,
            user.updated_at,
            user.last_login_at
        )
        .execute(&self.db)
        .await?;
        if result.rows_affected() > 1 {
            let event = UserCreated {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
            };
            self.save_event(event).await?
        }
        Ok(())
    }

    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error> {
        let payload = serde_json::to_value(&event).unwrap();
        sqlx::query!(
            "INSERT INTO events (id,payload) VALUES ($1,$2)",
            Uuid::now_v7(),
            payload
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<models::User>, sqlx::Error> {
        sqlx::query_as!(models::User, "SELECT * from users WHERE id = $1", id)
            .fetch_optional(&self.db)
            .await
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<models::User>, sqlx::Error> {
        sqlx::query_as!(models::User, "SELECT * from users WHERE email = $1", email)
            .fetch_optional(&self.db)
            .await
    }
}

#[async_trait]
impl UserProfileRepository for PostgreSQL {
    async fn save_profile(&self, profile: models::UserProfile) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO user_profiles (
                user_id, first_name, last_name, avatar_url, bio, preferences, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id) DO UPDATE SET
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                avatar_url = EXCLUDED.avatar_url,
                bio = EXCLUDED.bio,
                preferences = EXCLUDED.preferences,
                updated_at = EXCLUDED.updated_at
            "#,
            profile.user_id,
            profile.first_name,
            profile.last_name,
            profile.avatar_url,
            profile.bio,
            profile.preferences,
            profile.created_at,
            profile.updated_at
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn find_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<models::UserProfile>, sqlx::Error> {
        sqlx::query_as!(
            models::UserProfile,
            r#"
            SELECT user_id, first_name, last_name, avatar_url, bio, preferences, created_at, updated_at
            FROM user_profiles WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
    }

    async fn update_profile(&self, profile: models::UserProfile) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE user_profiles SET
                first_name = $2,
                last_name = $3,
                avatar_url = $4,
                bio = $5,
                preferences = $6,
                updated_at = $7
            WHERE user_id = $1
            "#,
            profile.user_id,
            profile.first_name,
            profile.last_name,
            profile.avatar_url,
            profile.bio,
            profile.preferences,
            profile.updated_at
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn delete_profile(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM user_profiles WHERE user_id = $1", user_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn profile_exists(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM user_profiles WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.db)
        .await?;
        Ok(result.count.unwrap_or(0) > 0)
    }
}