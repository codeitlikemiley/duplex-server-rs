use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserCreated {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
