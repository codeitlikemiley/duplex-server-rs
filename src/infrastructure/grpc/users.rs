use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login},
    proto::{
        CreateUserRequest, CreateUserResponse, GetProfileRequest, GetProfileResponse,
        GetUserRequest, GetUserResponse, LoginRequest, LoginResponse,
        user_service_server::{UserService as GrpcUserService, UserServiceServer},
    },
    services::UserService,
};

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
        let id_str = request.into_inner().id;

        let id = match Uuid::parse_str(&id_str) {
            Ok(uuid) => uuid,
            Err(_) => return Err(Status::invalid_argument("Invalid user ID format")),
        };

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
                Err(Status::internal("Database error"))
            }
        }
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let login_req = request.into_inner();
        let command = Login {
            email: login_req.email,
            password: login_req.password,
        };

        match self.repo.handle_login(command).await {
            Ok(token) => {
                info!("Login successful, token: {}", token);
                Ok(Response::new(LoginResponse { token }))
            }
            Err(_) => {
                error!("Login failed");
                Err(Status::unauthenticated("Invalid credentials"))
            }
        }
    }

    async fn get_profile(
        &self,
        request: Request<GetProfileRequest>,
    ) -> Result<Response<GetProfileResponse>, Status> {
        let jwt_service = crate::infrastructure::auth::JwtService::default();

        // Extract token from authorization metadata
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|auth_header| {
                if auth_header.starts_with("Bearer ") {
                    Some(auth_header.strip_prefix("Bearer ").unwrap_or(""))
                } else {
                    None
                }
            });

        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Err(Status::unauthenticated("Missing or invalid authorization header"));
            }
        };

        // Verify token
        let claims = match jwt_service.verify_token(token) {
            Ok(claims) => claims,
            Err(e) => {
                tracing::warn!("Invalid JWT token: {:?}", e);
                return Err(Status::unauthenticated("Invalid token"));
            }
        };

        info!("Profile accessed for user: {}", claims.sub);

        Ok(Response::new(GetProfileResponse {
            user_id: claims.sub.clone(),
            email: claims.email.clone(),
            message: "This is a protected gRPC endpoint".to_string(),
        }))
    }
}
