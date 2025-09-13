use axum::{
    middleware,
    routing::{get, post, Router as HttpRouter},
    Router,
};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;

use crate::{commands::CommandMessage, services::UserService, Api, PostgreSQL};

use super::{controllers::{create_user, get_user_by_id, login, get_profile}, middleware::auth_middleware};

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
