use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands::CreateUser,
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
    pub fn new(pool: Pool<Postgres>) -> UserServiceServer<GrpcUserServiceImpl> {
        let user_service = UserService::new(PostgreSQL::new(pool.clone()));
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

        match self.repo.handle_create_user(command).await {
            Ok(_) => {
                info!("User Created");
                Ok(Response::new(CreateUserResponse {}))
            }
            Err(e) => {
                error!("{}", e);
                Err(Status::already_exists("User already Exists"))
            }
        }
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
