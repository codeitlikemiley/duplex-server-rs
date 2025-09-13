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
        .build_v1alpha()
        .unwrap();

    // Use Routes to create an axum router
    tonic::service::Routes::new(reflection_service)
        .add_service(GrpcUserServiceImpl::new(pool.clone(), sender))
        .into_axum_router()
}
