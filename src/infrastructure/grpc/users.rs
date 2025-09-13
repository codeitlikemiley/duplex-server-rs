//! gRPC User Service implementation with standardized error handling
//!
//! This service translates domain AppError instances to gRPC Status responses
//! for consistent error handling across the API.

use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tracing;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login},
    errors::AppError,
    infrastructure::errors::ErrorTranslator,
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
        let req = request.into_inner();
        let command = CreateUser::from(req.clone());

        // Handle user creation with proper error conversion
        match self.repo.handle_create_user(command).await {
            Ok(()) => {
                tracing::info!(
                    "CreateUser: Successfully created user: {} ({})",
                    req.username,
                    req.email
                );
                Ok(Response::new(CreateUserResponse {}))
            }
            Err(app_error) => {
                tracing::warn!(
                    "CreateUser: Failed to create user {}: {:?}",
                    req.username,
                    app_error
                );
                Err(ErrorTranslator::to_grpc_status(app_error.into()))
            }
        }
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let id_str = request.into_inner().id;

        let id = match Uuid::parse_str(&id_str) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error = AppError::Validation {
                    field: "id".to_string(),
                    message: "Invalid UUID format".to_string(),
                };
                tracing::warn!("GetUser: Invalid UUID format: {}", id_str);
                return Err(ErrorTranslator::to_grpc_status(error));
            }
        };

        match self.repo.handle_get_user_by_id(id).await {
            Ok(Some(user)) => {
                tracing::info!("GetUser: Found user {} ({})", user.username, user.email);
                let response = Response::new(GetUserResponse {
                    id: user.id.to_string(),
                    email: user.email,
                    username: user.username,
                });
                Ok(response)
            }
            Ok(None) => {
                let error = AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(id_str.clone()),
                };
                tracing::warn!("GetUser: User not found with ID: {}", id_str);
                Err(ErrorTranslator::to_grpc_status(error))
            }
            Err(app_error) => {
                tracing::error!("GetUser: Database error for ID {}: {:?}", id_str, app_error);
                Err(ErrorTranslator::to_grpc_status(app_error.into()))
            }
        }
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let login_req = request.into_inner();
        let email_clone = login_req.email.clone();
        let command = Login {
            email: login_req.email,
            password: login_req.password,
        };

        match self.repo.handle_login(command).await {
            Ok(token) => {
                tracing::info!("Login: Successful login for user: {}", email_clone);
                Ok(Response::new(LoginResponse { token }))
            }
            Err(app_error) => {
                tracing::warn!("Login: Failed login attempt for user: {}", email_clone);
                Err(ErrorTranslator::to_grpc_status(app_error.into()))
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
                let error = AppError::Authentication {
                    message: "Missing or invalid authorization header. Use format: 'authorization: Bearer <token>'".to_string(),
                };
                tracing::warn!("GetProfile: Missing or invalid authorization header");
                return Err(ErrorTranslator::to_grpc_status(error));
            }
        };

        // Verify token
        let claims = match jwt_service.verify_token(token) {
            Ok(claims) => claims,
            Err(e) => {
                let error = AppError::Authentication {
                    message: "Invalid or expired JWT token. Please login again.".to_string(),
                };
                tracing::warn!("GetProfile: Invalid JWT token - {:?}", e);
                return Err(ErrorTranslator::to_grpc_status(error));
            }
        };

        tracing::info!(
            "GetProfile: Successfully accessed profile for user: {} ({})",
            claims.sub,
            claims.email
        );

        Ok(Response::new(GetProfileResponse {
            user_id: claims.sub.clone(),
            email: claims.email.clone(),
            message: "✅ Profile accessed successfully - this is a protected gRPC endpoint"
                .to_string(),
        }))
    }
}
