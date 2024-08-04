use uuid::Uuid;

use crate::{
    commands::CreateUser, events::UserCreated, models::User, persistence::PostgreSQL,
    repositories::UserRepository,
};

#[derive(Clone)]
pub struct UserService {
    pub repo: PostgreSQL,
}

impl UserService {
    pub fn new(repo: PostgreSQL) -> Self {
        Self { repo }
    }

    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), sqlx::Error> {
        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
        };

        let event = UserCreated {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
        };

        self.repo.save_user(user).await?;
        self.repo.save_event(event).await?;

        Ok(())
    }

    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        self.repo.find_user_by_id(id).await
    }
}
