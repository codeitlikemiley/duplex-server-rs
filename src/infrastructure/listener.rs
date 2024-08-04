use tokio::net::TcpListener;

pub async fn listener() -> TcpListener {
    tokio::net::TcpListener::bind("[::]:80").await.unwrap()
}
