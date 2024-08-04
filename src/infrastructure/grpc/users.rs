use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands::{CommandMessage, CreateUser},
    proto::{
        user_service_server::{UserService as GrpcUserService, UserServiceServer},
        CreateUserRequest, CreateUserResponse, GetUserRequest, GetUserResponse,
    },
    services::UserService,
    PostgreSQL,
};

#[derive(Debug)]
pub struct GrpcUserServiceImpl {
    repo: UserService,
}

impl GrpcUserServiceImpl {
    pub fn new(
        pool: Pool<Postgres>,
        sender: mpsc::Sender<CommandMessage>,
    ) -> UserServiceServer<GrpcUserServiceImpl> {
        let user_service = UserService::new(PostgreSQL::new(pool.clone()), sender.clone());
        UserServiceServer::new(GrpcUserServiceImpl { repo: user_service })
    }
}

#[tonic::async_trait]
impl GrpcUserService for GrpcUserServiceImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let command = CreateUser::from(request.into_inner());

        self.repo.create_user(command).await;
        Ok(Response::new(CreateUserResponse {}))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let id = Uuid::parse_str(&request.into_inner().id).unwrap();

        match self.repo.handle_get_user_by_id(id).await {
            Ok(Some(user)) => {
                info!("User Found:\n{:#?}", user);
                let response = Response::new(GetUserResponse {
                    id: user.id.to_string(),
                    email: user.email,
                    username: user.username,
                });

                Ok(response)
            }
            Ok(None) => Err(Status::not_found("User Not Found")),
            Err(e) => {
                error!("{}", e);
                Err(Status::not_found("User Not Found"))
            }
        }
    }
}
