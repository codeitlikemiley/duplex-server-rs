mod commands;
mod db;
mod events;
mod handlers;
mod models;
mod repositories;
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use handlers::UserHandler;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "coqrs=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv::dotenv().ok();

    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/coqrs".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let state = UserHandler::new(db::PgPool::new(pool.clone()));

    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user_by_id))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn create_user(
    State(handler): State<UserHandler>,
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

async fn get_user_by_id(
    State(state): State<handlers::UserHandler>,
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
