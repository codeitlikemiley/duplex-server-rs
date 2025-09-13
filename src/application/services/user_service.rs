use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login, send_command},
    models::User,
    repositories::UserRepository,
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
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(cmd.password.as_bytes(), &salt)
            .map_err(|_| sqlx::Error::RowNotFound)?
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

    pub async fn handle_get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        self.repo.find_user_by_id(id).await
    }

    pub async fn handle_login(&self, cmd: Login) -> Result<String, sqlx::Error> {
        // For now, just return a dummy token. In production, verify password and return JWT
        Ok(Uuid::now_v7().to_string())
    }

    pub async fn create_user(&self, cmd: CreateUser) {
        send_command(self.sender.clone(), CommandMessage::CreateUser(cmd)).await;
    }

    pub async fn login(&self, cmd: Login) {
        send_command(self.sender.clone(), CommandMessage::Login(cmd)).await;
    }
}
