use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{commands, services::UserService};

pub async fn create_user(
    State(handler): State<UserService>,
    Json(payload): Json<commands::CreateUser>,
) -> impl IntoResponse {
    match handler.handle_create_user(payload).await {
        Ok(_) => {
            info!("User Created");
            "User created".into_response()
        }
        Err(_) => {
            error!("Failed to Create User");
            "Failed to create user".into_response()
        }
    }
}
pub async fn get_user_by_id(
    State(state): State<UserService>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.handle_get_user_by_id(id).await {
        Ok(Some(user)) => {
            info!("User Found:\n {:#?}", user);
            Json(user).into_response()
        }
        Ok(None) => {
            info!("User Not Found");
            "User not found".into_response()
        }
        Err(_) => {
            error!("Failed to Fetch User");
            "Failed to get user".into_response()
        }
    }
}
