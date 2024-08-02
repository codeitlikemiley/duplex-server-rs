use axum::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{events::UserCreated, models, repositories::UserRepository};

#[derive(Clone)]
pub struct PgPool {
    db: Pool<Postgres>,
}

impl PgPool {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for PgPool {
    async fn save_user(&self, user: models::User) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO users (id,username,email) VALUES ($1,$2,$3)")
            .bind(user.id)
            .bind(&user.username)
            .bind(&user.email)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error> {
        let payload = serde_json::to_value(&event).unwrap();
        sqlx::query("INSERT INTO events (id,payload) VALUES ($1,$2)")
            .bind(Uuid::now_v7())
            .bind(payload)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<models::User>, sqlx::Error> {
        sqlx::query_as::<_, models::User>("SELECT * from users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await
    }
}
