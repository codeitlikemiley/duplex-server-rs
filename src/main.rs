use axum::{extract::Request, http::header::CONTENT_TYPE};
use coqrs::{db, init_logger, router, services};
use tower::{make::Shared, steer::Steer};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenvy::dotenv()?;

    init_logger();

    let pool = db::pgpool_connections().await;

    let lb = Steer::new(
        [router(pool.clone()), services(pool)],
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
