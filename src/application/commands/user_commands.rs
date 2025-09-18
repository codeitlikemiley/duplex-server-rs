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

#[derive(Deserialize, Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_create_user_creation() {
        let cmd = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(cmd.username, "testuser");
        assert_eq!(cmd.email, "test@example.com");
        assert_eq!(cmd.password, "password123");
    }

    #[test]
    fn test_create_user_serialization() {
        let cmd = CreateUser {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let json = serde_json::to_string(&cmd).expect("Failed to serialize CreateUser");
        let deserialized: CreateUser = serde_json::from_str(&json).expect("Failed to deserialize CreateUser");

        assert_eq!(deserialized.username, cmd.username);
        assert_eq!(deserialized.email, cmd.email);
        assert_eq!(deserialized.password, cmd.password);
    }

    #[test]
    fn test_register_user_creation() {
        let cmd = RegisterUser {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "securepass".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
        };

        assert_eq!(cmd.username, "newuser");
        assert_eq!(cmd.email, "new@example.com");
        assert_eq!(cmd.password, "securepass");
        assert_eq!(cmd.first_name, Some("John".to_string()));
        assert_eq!(cmd.last_name, Some("Doe".to_string()));
    }

    #[test]
    fn test_register_user_optional_fields() {
        let cmd = RegisterUser {
            username: "minimaluser".to_string(),
            email: "minimal@example.com".to_string(),
            password: "password".to_string(),
            first_name: None,
            last_name: None,
        };

        assert_eq!(cmd.username, "minimaluser");
        assert_eq!(cmd.email, "minimal@example.com");
        assert_eq!(cmd.password, "password");
        assert!(cmd.first_name.is_none());
        assert!(cmd.last_name.is_none());
    }

    #[test]
    fn test_register_user_serialization() {
        let cmd = RegisterUser {
            username: "serialuser".to_string(),
            email: "serial@example.com".to_string(),
            password: "testpass".to_string(),
            first_name: Some("Jane".to_string()),
            last_name: None,
        };

        let json = serde_json::to_string(&cmd).expect("Failed to serialize RegisterUser");
        let deserialized: RegisterUser = serde_json::from_str(&json).expect("Failed to deserialize RegisterUser");

        assert_eq!(deserialized.username, cmd.username);
        assert_eq!(deserialized.email, cmd.email);
        assert_eq!(deserialized.password, cmd.password);
        assert_eq!(deserialized.first_name, cmd.first_name);
        assert_eq!(deserialized.last_name, cmd.last_name);
    }

    #[test]
    fn test_login_creation() {
        let cmd = Login {
            email: "login@example.com".to_string(),
            password: "loginpass".to_string(),
        };

        assert_eq!(cmd.email, "login@example.com");
        assert_eq!(cmd.password, "loginpass");
    }

    #[test]
    fn test_login_clone() {
        let cmd = Login {
            email: "clone@example.com".to_string(),
            password: "clonepass".to_string(),
        };

        let cloned = cmd.clone();
        assert_eq!(cloned.email, cmd.email);
        assert_eq!(cloned.password, cmd.password);
    }

    #[test]
    fn test_login_serialization() {
        let cmd = Login {
            email: "serialize@example.com".to_string(),
            password: "serializepass".to_string(),
        };

        let json = serde_json::to_string(&cmd).expect("Failed to serialize Login");
        let deserialized: Login = serde_json::from_str(&json).expect("Failed to deserialize Login");

        assert_eq!(deserialized.email, cmd.email);
        assert_eq!(deserialized.password, cmd.password);
    }

    #[test]
    fn test_verify_email_creation() {
        let user_id = Uuid::now_v7();
        let cmd = VerifyEmail {
            user_id,
            verification_token: "abc123def456".to_string(),
        };

        assert_eq!(cmd.user_id, user_id);
        assert_eq!(cmd.verification_token, "abc123def456");
    }

    #[test]
    fn test_verify_email_serialization() {
        let user_id = Uuid::now_v7();
        let cmd = VerifyEmail {
            user_id,
            verification_token: "token123".to_string(),
        };

        let json = serde_json::to_string(&cmd).expect("Failed to serialize VerifyEmail");
        let deserialized: VerifyEmail = serde_json::from_str(&json).expect("Failed to deserialize VerifyEmail");

        assert_eq!(deserialized.user_id, cmd.user_id);
        assert_eq!(deserialized.verification_token, cmd.verification_token);
    }

    #[test]
    fn test_register_user_to_create_user_conversion() {
        let register_cmd = RegisterUser {
            username: "converter".to_string(),
            email: "convert@example.com".to_string(),
            password: "convertpass".to_string(),
            first_name: Some("Convert".to_string()),
            last_name: Some("User".to_string()),
        };

        let create_cmd: CreateUser = register_cmd.into();

        assert_eq!(create_cmd.username, "converter");
        assert_eq!(create_cmd.email, "convert@example.com");
        assert_eq!(create_cmd.password, "convertpass");
    }

    #[test]
    fn test_command_message_variants() {
        let create_user = CreateUser {
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            password: "pass".to_string(),
        };

        let register_user = RegisterUser {
            username: "reg".to_string(),
            email: "reg@example.com".to_string(),
            password: "regpass".to_string(),
            first_name: None,
            last_name: None,
        };

        let login = Login {
            email: "login@example.com".to_string(),
            password: "loginpass".to_string(),
        };

        let verify_email = VerifyEmail {
            user_id: Uuid::now_v7(),
            verification_token: "token".to_string(),
        };

        let cmd_create = CommandMessage::CreateUser(create_user);
        let cmd_register = CommandMessage::RegisterUser(register_user);
        let cmd_login = CommandMessage::Login(login);
        let cmd_verify = CommandMessage::VerifyEmail(verify_email);

        match cmd_create {
            CommandMessage::CreateUser(_) => (),
            _ => panic!("Expected CreateUser variant"),
        }

        match cmd_register {
            CommandMessage::RegisterUser(_) => (),
            _ => panic!("Expected RegisterUser variant"),
        }

        match cmd_login {
            CommandMessage::Login(_) => (),
            _ => panic!("Expected Login variant"),
        }

        match cmd_verify {
            CommandMessage::VerifyEmail(_) => (),
            _ => panic!("Expected VerifyEmail variant"),
        }
    }

    #[tokio::test]
    async fn test_command_handler_creation() {
        let (tx, rx) = mpsc::channel(100);
        let handler = CommandHandler::new(rx);

        assert!(std::mem::size_of_val(&handler) > 0);
        drop(tx);
    }

    #[tokio::test]
    async fn test_send_command_success() {
        let (tx, mut rx) = mpsc::channel(1);

        let command = CommandMessage::CreateUser(CreateUser {
            username: "sendtest".to_string(),
            email: "send@test.com".to_string(),
            password: "sendpass".to_string(),
        });

        tokio::spawn(async move {
            send_command(tx, command).await;
        });

        let received = rx.recv().await.expect("Should receive command");
        match received {
            CommandMessage::CreateUser(cmd) => {
                assert_eq!(cmd.username, "sendtest");
                assert_eq!(cmd.email, "send@test.com");
                assert_eq!(cmd.password, "sendpass");
            }
            _ => panic!("Expected CreateUser command"),
        }
    }

    #[tokio::test]
    async fn test_send_command_channel_closed() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);

        let command = CommandMessage::Login(Login {
            email: "closed@test.com".to_string(),
            password: "closedpass".to_string(),
        });

        send_command(tx, command).await;
    }

    #[test]
    fn test_empty_string_fields() {
        let cmd = CreateUser {
            username: "".to_string(),
            email: "".to_string(),
            password: "".to_string(),
        };

        assert_eq!(cmd.username, "");
        assert_eq!(cmd.email, "");
        assert_eq!(cmd.password, "");
    }

    #[test]
    fn test_unicode_content() {
        let cmd = RegisterUser {
            username: "用户名".to_string(),
            email: "test@тест.com".to_string(),
            password: "пароль123".to_string(),
            first_name: Some("名前".to_string()),
            last_name: Some("Фамилия".to_string()),
        };

        assert_eq!(cmd.username, "用户名");
        assert_eq!(cmd.email, "test@тест.com");
        assert_eq!(cmd.password, "пароль123");
        assert_eq!(cmd.first_name, Some("名前".to_string()));
        assert_eq!(cmd.last_name, Some("Фамилия".to_string()));
    }

    #[test]
    fn test_special_characters() {
        let cmd = Login {
            email: "test+tag@example-domain.co.uk".to_string(),
            password: "P@ssw0rd!@#$%^&*()".to_string(),
        };

        assert_eq!(cmd.email, "test+tag@example-domain.co.uk");
        assert_eq!(cmd.password, "P@ssw0rd!@#$%^&*()");
    }

    #[test]
    fn test_verify_email_with_long_token() {
        let cmd = VerifyEmail {
            user_id: Uuid::now_v7(),
            verification_token: "a".repeat(1000),
        };

        assert_eq!(cmd.verification_token.len(), 1000);
        assert!(cmd.verification_token.chars().all(|c| c == 'a'));
    }
}
