use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    commands::{send_command, CommandMessage, CreateUser},
    models::User,
    repositories::UserRepository,
    PostgreSQL,
};

#[derive(Clone, Debug)]
pub struct UserService {
    pub repo: PostgreSQL,
    pub sender: mpsc::Sender<CommandMessage>,
}

impl UserService {
    pub fn new(repo: PostgreSQL, sender: mpsc::Sender<CommandMessage>) -> Self {
        Self { repo, sender }
    }

    pub async fn handle_create_user(&self, cmd: CreateUser) -> Result<(), sqlx::Error> {
        let user = User {
            id: Uuid::now_v7(),
            username: cmd.username,
            email: cmd.email,
        };

        self.repo.save_user(user).await?;
        Ok(())
    }

    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        self.repo.find_user_by_id(id).await
    }

    pub async fn create_user(&self, cmd: CreateUser) {
        send_command(self.sender.clone(), CommandMessage::CreateUser(cmd)).await;
    }
}
