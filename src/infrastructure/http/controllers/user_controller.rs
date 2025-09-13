use axum::{
    extract::{Extension, Path, State},
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{commands, services::UserService, infrastructure::auth::Claims};

pub async fn create_user(
    State(handler): State<UserService>,
    Json(payload): Json<commands::CreateUser>,
) -> impl IntoResponse {
    handler.create_user(payload).await;
    "User creation initiated".into_response()
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

pub async fn login(
    State(state): State<UserService>,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    match state.handle_login(payload).await {
        Ok(token) => {
            info!("Login successful, token: {}", token);
            Json(serde_json::json!({"token": token})).into_response()
        }
        Err(_) => {
            error!("Login failed");
            "Login failed".into_response()
        }
    }
}

#[axum::debug_handler]
pub async fn get_profile(
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    info!("Profile accessed for user: {}", claims.sub);
    Json(serde_json::json!({
        "user_id": claims.sub,
        "email": claims.email,
        "message": "This is a protected endpoint"
    })).into_response()
}
