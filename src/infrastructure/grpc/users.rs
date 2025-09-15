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

                // Convert to proto User
                let proto_user = crate::proto::User {
                    id: user.id.to_string(),
                    username: user.username.clone(),
                    email: user.email.clone(),
                    email_verified: user.email_verified,
                    status: match user.status {
                        crate::models::UserStatus::Active => crate::proto::UserStatus::Active as i32,
                        crate::models::UserStatus::Inactive => crate::proto::UserStatus::Inactive as i32,
                        crate::models::UserStatus::Suspended => crate::proto::UserStatus::Suspended as i32,
                        crate::models::UserStatus::PendingVerification => crate::proto::UserStatus::PendingVerification as i32,
                    },
                    created_at: user.created_at.to_rfc3339(),
                    updated_at: user.updated_at.to_rfc3339(),
                    last_login_at: user.last_login_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                };

                let response = Response::new(GetUserResponse {
                    user: Some(proto_user),
                    roles: vec![], // TODO: Add roles when RBAC service is integrated
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
            email: login_req.email.clone(),
            password: login_req.password,
        };

        match self.repo.handle_login(command).await {
            Ok(token) => {
                // Get user info for the response
                let user = match self.repo.repo.find_user_by_email(&login_req.email).await {
                    Ok(Some(u)) => u,
                    _ => {
                        return Err(ErrorTranslator::to_grpc_status(AppError::Internal {
                            message: "Failed to retrieve user after login".to_string(),
                        }));
                    }
                };

                let proto_user = crate::proto::User {
                    id: user.id.to_string(),
                    username: user.username.clone(),
                    email: user.email.clone(),
                    email_verified: user.email_verified,
                    status: match user.status {
                        crate::models::UserStatus::Active => crate::proto::UserStatus::Active as i32,
                        crate::models::UserStatus::Inactive => crate::proto::UserStatus::Inactive as i32,
                        crate::models::UserStatus::Suspended => crate::proto::UserStatus::Suspended as i32,
                        crate::models::UserStatus::PendingVerification => crate::proto::UserStatus::PendingVerification as i32,
                    },
                    created_at: user.created_at.to_rfc3339(),
                    updated_at: user.updated_at.to_rfc3339(),
                    last_login_at: user.last_login_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                };

                tracing::info!("Login: Successful login for user: {}", email_clone);
                Ok(Response::new(LoginResponse {
                    token,
                    user: Some(proto_user),
                }))
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

        // For now, return a simple profile response
        // TODO: Actually fetch the user's profile from the database
        Ok(Response::new(GetProfileResponse {
            profile: Some(crate::proto::UserProfile {
                user_id: claims.sub.clone(),
                first_name: String::new(),
                last_name: String::new(),
                bio: String::new(),
                location: String::new(),
                website: String::new(),
                avatar_url: String::new(),
                preferences_json: String::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            }),
        }))
    }

    // Stub implementations for remaining methods - TODO: Implement these
    async fn register_user(
        &self,
        _request: Request<crate::proto::RegisterUserRequest>,
    ) -> Result<Response<crate::proto::RegisterUserResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn logout(
        &self,
        _request: Request<crate::proto::LogoutRequest>,
    ) -> Result<Response<crate::proto::LogoutResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn logout_all_devices(
        &self,
        _request: Request<crate::proto::LogoutAllDevicesRequest>,
    ) -> Result<Response<crate::proto::LogoutAllDevicesResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn verify_email(
        &self,
        _request: Request<crate::proto::VerifyEmailRequest>,
    ) -> Result<Response<crate::proto::VerifyEmailResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_current_user(
        &self,
        _request: Request<crate::proto::GetCurrentUserRequest>,
    ) -> Result<Response<crate::proto::GetCurrentUserResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn update_profile(
        &self,
        _request: Request<crate::proto::UpdateProfileRequest>,
    ) -> Result<Response<crate::proto::UpdateProfileResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn update_account(
        &self,
        _request: Request<crate::proto::UpdateAccountRequest>,
    ) -> Result<Response<crate::proto::UpdateAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn delete_profile(
        &self,
        _request: Request<crate::proto::DeleteProfileRequest>,
    ) -> Result<Response<crate::proto::DeleteProfileResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn change_password(
        &self,
        _request: Request<crate::proto::ChangePasswordRequest>,
    ) -> Result<Response<crate::proto::ChangePasswordResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn request_password_reset(
        &self,
        _request: Request<crate::proto::RequestPasswordResetRequest>,
    ) -> Result<Response<crate::proto::RequestPasswordResetResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn reset_password(
        &self,
        _request: Request<crate::proto::ResetPasswordRequest>,
    ) -> Result<Response<crate::proto::ResetPasswordResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn deactivate_account(
        &self,
        _request: Request<crate::proto::DeactivateAccountRequest>,
    ) -> Result<Response<crate::proto::DeactivateAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn reactivate_account(
        &self,
        _request: Request<crate::proto::ReactivateAccountRequest>,
    ) -> Result<Response<crate::proto::ReactivateAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn delete_account(
        &self,
        _request: Request<crate::proto::DeleteAccountRequest>,
    ) -> Result<Response<crate::proto::DeleteAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_account_status(
        &self,
        _request: Request<crate::proto::GetAccountStatusRequest>,
    ) -> Result<Response<crate::proto::GetAccountStatusResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn search_users(
        &self,
        _request: Request<crate::proto::SearchUsersRequest>,
    ) -> Result<Response<crate::proto::SearchUsersResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn advanced_search(
        &self,
        _request: Request<crate::proto::AdvancedSearchRequest>,
    ) -> Result<Response<crate::proto::AdvancedSearchResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn suggest_users(
        &self,
        _request: Request<crate::proto::SuggestUsersRequest>,
    ) -> Result<Response<crate::proto::SuggestUsersResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_public_directory(
        &self,
        _request: Request<crate::proto::GetPublicDirectoryRequest>,
    ) -> Result<Response<crate::proto::GetPublicDirectoryResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_user_activities(
        &self,
        _request: Request<crate::proto::GetUserActivitiesRequest>,
    ) -> Result<Response<crate::proto::GetUserActivitiesResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_activity_stats(
        &self,
        _request: Request<crate::proto::GetActivityStatsRequest>,
    ) -> Result<Response<crate::proto::GetActivityStatsResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_active_sessions(
        &self,
        _request: Request<crate::proto::GetActiveSessionsRequest>,
    ) -> Result<Response<crate::proto::GetActiveSessionsResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn revoke_session(
        &self,
        _request: Request<crate::proto::RevokeSessionRequest>,
    ) -> Result<Response<crate::proto::RevokeSessionResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn lock_user_account(
        &self,
        _request: Request<crate::proto::LockUserAccountRequest>,
    ) -> Result<Response<crate::proto::LockUserAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn unlock_user_account(
        &self,
        _request: Request<crate::proto::UnlockUserAccountRequest>,
    ) -> Result<Response<crate::proto::UnlockUserAccountResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_lockout_status(
        &self,
        _request: Request<crate::proto::GetLockoutStatusRequest>,
    ) -> Result<Response<crate::proto::GetLockoutStatusResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_lockout_history(
        &self,
        _request: Request<crate::proto::GetLockoutHistoryRequest>,
    ) -> Result<Response<crate::proto::GetLockoutHistoryResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn assign_role(
        &self,
        _request: Request<crate::proto::AssignRoleRequest>,
    ) -> Result<Response<crate::proto::AssignRoleResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn remove_role(
        &self,
        _request: Request<crate::proto::RemoveRoleRequest>,
    ) -> Result<Response<crate::proto::RemoveRoleResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_user_roles(
        &self,
        _request: Request<crate::proto::GetUserRolesRequest>,
    ) -> Result<Response<crate::proto::GetUserRolesResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_user_permissions(
        &self,
        _request: Request<crate::proto::GetUserPermissionsRequest>,
    ) -> Result<Response<crate::proto::GetUserPermissionsResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn check_permission(
        &self,
        _request: Request<crate::proto::CheckPermissionRequest>,
    ) -> Result<Response<crate::proto::CheckPermissionResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }
}
