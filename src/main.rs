use coqrs::{
    PostgreSQL, commands::CommandHandler, db, init_logger, router, services::UserService,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    init_logger();

    // Create a channel for sending commands with a buffer size of 32
    let (sender, receiver) = mpsc::channel(32);

    let pool = db::pgpool_connections().await;

    let user_service = UserService::new(PostgreSQL::new(pool.clone()), sender.clone());

    let handler = CommandHandler::new(receiver);

    tokio::spawn(handler.run(user_service.clone()));

    let app = router(pool.clone(), sender.clone());

    let listener = tokio::net::TcpListener::bind("[::]:80").await.unwrap();

    tracing::debug!("listening on {:?}", listener.local_addr().unwrap());

    let server = axum::serve(listener, app).await;

    if let Err(err) = server {
        tracing::error!("server error: {:?}", err);
    }

    Ok(())
}
