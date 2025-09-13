use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{domain::Command, proto::CreateUserRequest, services::UserService};

#[derive(Deserialize, Debug)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct VerifyEmail {
    pub user_id: Uuid,
    pub verification_token: String,
}

impl Command for CreateUser {}
impl Command for RegisterUser {}
impl Command for Login {}
impl Command for VerifyEmail {}

impl From<CreateUserRequest> for CreateUser {
    fn from(value: CreateUserRequest) -> Self {
        CreateUser {
            email: value.email,
            username: value.username,
            password: value.password,
        }
    }
}

impl From<RegisterUser> for CreateUser {
    fn from(value: RegisterUser) -> Self {
        CreateUser {
            email: value.email,
            username: value.username,
            password: value.password,
        }
    }
}

pub enum CommandMessage {
    CreateUser(CreateUser),
    RegisterUser(RegisterUser),
    Login(Login),
    VerifyEmail(VerifyEmail),
}

pub struct CommandHandler {
    receiver: mpsc::Receiver<CommandMessage>,
}

impl CommandHandler {
    pub fn new(receiver: mpsc::Receiver<CommandMessage>) -> Self {
        CommandHandler { receiver }
    }

    pub async fn run(self, user_service: UserService) {
        let mut receiver = self.receiver;
        while let Some(command) = receiver.recv().await {
            match command {
                CommandMessage::CreateUser(cmd) => {
                    if let Err(e) = user_service.handle_create_user(cmd).await {
                        //TODO: Add error Hander and bubble up this error to send 400
                        tracing::error!("Failed to handle CreateUser command: {}", e);
                    }
                }
                CommandMessage::RegisterUser(cmd) => {
                    if let Err(e) = user_service.handle_register_user(cmd).await {
                        tracing::error!("Failed to handle RegisterUser command: {}", e);
                    }
                }
                CommandMessage::Login(cmd) => {
                    if let Err(e) = user_service.handle_login(cmd).await {
                        tracing::error!("Failed to handle Login command: {}", e);
                    }
                }
                CommandMessage::VerifyEmail(cmd) => {
                    if let Err(e) = user_service.handle_verify_email(cmd).await {
                        tracing::error!("Failed to handle VerifyEmail command: {}", e);
                    }
                }
            }
        }
    }
}

pub async fn send_command(sender: mpsc::Sender<CommandMessage>, command: CommandMessage) {
    if sender.send(command).await.is_err() {
        eprintln!("Failed to send command");
    }
}
