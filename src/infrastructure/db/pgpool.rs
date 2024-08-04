use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub async fn pgpool_connections() -> Pool<Postgres> {
    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/coqrs".to_string());

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database")
}
