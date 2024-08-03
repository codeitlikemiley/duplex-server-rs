mod commands;
mod controllers;
mod db;
mod domain;
mod events;
mod models;
mod repositories;
mod routes;
mod services;
use axum::{
    routing::{get, post},
    Router,
};
use controllers::{create_user, get_user_by_id};
use routes::Api;
use services::UserService;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

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

    let user_service = UserService::new(db::PgPool::new(pool.clone()));

    let app = Router::new()
        .route(Api::CreateUser.into(), post(create_user))
        .route(Api::GetUser.into(), get(get_user_by_id))
        .with_state(user_service);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
