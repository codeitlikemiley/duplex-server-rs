use axum::{extract::Request, http::header::CONTENT_TYPE};
use coqrs::{
    commands::CommandHandler, db, grpc_services, init_logger, router, services::UserService,
    PostgreSQL,
};
use tokio::sync::mpsc;
use tower::{make::Shared, steer::Steer};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenvy::dotenv()?;

    init_logger();

    // Create a channel for sending commands with a buffer size of 32
    let (sender, receiver) = mpsc::channel(32);

    let pool = db::pgpool_connections().await;

    let user_service = UserService::new(PostgreSQL::new(pool.clone()), sender.clone());

    let handler = CommandHandler::new(receiver);

    tokio::spawn(handler.run(user_service.clone()));

    let lb = Steer::new(
        [
            router(pool.clone(), sender.clone()),
            grpc_services(pool.clone(), sender.clone()),
        ],
        |req: &Request, _services: &[_]| {
            req.headers()
                .get(CONTENT_TYPE)
                .map(|content_type| content_type.as_bytes())
                .filter(|content_type| content_type.starts_with(b"application/grpc"))
                .map(|_| 1)
                .unwrap_or(0)
        },
    );

    let listener = tokio::net::TcpListener::bind("[::]:80").await.unwrap();

    tracing::debug!("listening on {:?}", listener.local_addr().unwrap());

    let server = axum::serve(listener, Shared::new(lb)).await;

    if let Err(err) = server {
        tracing::error!("server error: {:?}", err);
    }

    Ok(())
}
