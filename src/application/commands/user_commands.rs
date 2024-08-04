use serde::Deserialize;

use crate::{domain::Command, proto::CreateUserRequest};

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
