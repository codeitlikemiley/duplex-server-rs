use axum::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{events::UserCreated, models, repositories::UserRepository};

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
        sqlx::query!(
            "INSERT INTO users (id,username,email) VALUES ($1,$2,$3)",
            user.id,
            &user.username,
            &user.email,
        )
        .execute(&self.db)
        .await?;
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
}
