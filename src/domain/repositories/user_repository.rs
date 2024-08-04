use axum::async_trait;
use uuid::Uuid;

use crate::{events::UserCreated, models::User};

#[async_trait]
pub trait UserRepository {
    async fn save_user(&self, user: User) -> Result<(), sqlx::Error>;
    async fn save_event(&self, event: UserCreated) -> Result<(), sqlx::Error>;
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
}
