use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Event;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserCreated {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
impl Event for UserCreated {}
