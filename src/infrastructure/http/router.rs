use axum::{
    Router, middleware,
    routing::{Router as HttpRouter, get, post},
};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;

use crate::{Api, PostgreSQL, commands::CommandMessage, services::UserService};

use super::{
    controllers::{create_user, get_profile, get_user_by_id, login},
    middleware::auth_middleware,
};

pub fn router(pool: Pool<Postgres>, sender: mpsc::Sender<CommandMessage>) -> HttpRouter {
    let user_service = UserService::new(PostgreSQL::new(pool.clone()), sender);

    let public_routes = Router::new()
        .route(Api::CreateUser.into(), post(create_user))
        .route(Api::GetUser.into(), get(get_user_by_id))
        .route(Api::Login.into(), post(login));

    let protected_routes = Router::new()
        .route(Api::Profile.into(), get(get_profile))
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(user_service)
}
