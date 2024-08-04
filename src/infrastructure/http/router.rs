use axum::{
    routing::{get, post, Router as HttpRouter},
    Router,
};
use sqlx::{Pool, Postgres};

use crate::{services::UserService, Api, PostgreSQL};

use super::controllers::{create_user, get_user_by_id};

pub fn router(pool: Pool<Postgres>) -> HttpRouter {
    let user_service = UserService::new(PostgreSQL::new(pool.clone()));
    Router::new()
        .route(Api::CreateUser.into(), post(create_user))
        .route(Api::GetUser.into(), get(get_user_by_id))
        .with_state(user_service)
}
