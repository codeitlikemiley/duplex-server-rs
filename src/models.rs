use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Model;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

impl Model for User {}
