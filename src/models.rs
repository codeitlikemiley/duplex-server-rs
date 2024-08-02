use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}
