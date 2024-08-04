use coqrs::{db, init_logger, listener, router};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenvy::dotenv()?;

    init_logger();

    let pool = db::pgpool_connections().await;

    let app = router(pool);

    let listener = listener().await;

    axum::serve(listener, app).await?;

    Ok(())
}
