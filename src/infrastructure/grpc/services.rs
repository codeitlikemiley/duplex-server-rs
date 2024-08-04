use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic_reflection::pb::v1alpha::FILE_DESCRIPTOR_SET;

use crate::commands::CommandMessage;

use super::users::GrpcUserServiceImpl;

pub fn services(
    pool: Pool<Postgres>,
    sender: mpsc::Sender<CommandMessage>,
) -> axum::routing::Router {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(reflection_service)
        .add_service(tonic_web::enable(GrpcUserServiceImpl::new(
            pool.clone(),
            sender,
        )))
        .into_router()
}
