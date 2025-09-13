use axum::{
    Router, middleware,
    routing::{Router as HttpRouter, get, post},
};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;

use crate::{Api, PostgreSQL, commands::CommandMessage, services::UserService};

use super::{
    controllers::{create_user, get_profile, get_user_by_id, login, register_user, verify_email, change_password},
    middleware::auth_middleware,
};

pub fn router(pool: Pool<Postgres>, sender: mpsc::Sender<CommandMessage>) -> HttpRouter {
    let postgres_db = PostgreSQL::new(pool.clone());
    let user_service = UserService::new(postgres_db.clone(), sender);

    let public_routes = Router::new()
        .route(Api::CreateUser.into(), post(create_user))
        .route(Api::GetUser.into(), get(get_user_by_id))
        .route(Api::Login.into(), post(login))
        .route("/api/register", post(register_user))
        .route("/api/verify-email", post(verify_email))
        .with_state(user_service);

    let protected_routes = Router::new()
        .route(Api::Profile.into(), get(get_profile))
        .route("/api/change-password", post(change_password))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(postgres_db);

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
}
