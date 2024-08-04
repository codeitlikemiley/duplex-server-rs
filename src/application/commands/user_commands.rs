use serde::Deserialize;
use tokio::sync::mpsc;

use crate::{domain::Command, proto::CreateUserRequest, services::UserService};

#[derive(Deserialize, Debug)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
}

impl Command for CreateUser {}

impl From<CreateUserRequest> for CreateUser {
    fn from(value: CreateUserRequest) -> Self {
        CreateUser {
            email: value.email,
            username: value.username,
        }
    }
}

pub enum CommandMessage {
    CreateUser(CreateUser),
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
                        tracing::error!("Failed to handle CreateUser command: {}", e);
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
