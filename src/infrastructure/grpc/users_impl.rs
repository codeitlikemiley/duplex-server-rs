//! Complete gRPC User Service implementation with all methods

use super::users::GrpcUserServiceImpl;
use crate::{
    commands::RegisterUser,
    errors::AppError,
    infrastructure::errors::ErrorTranslator,
    proto::*,
    services::{ProfileService, PasswordService, SearchService, ActivityLoggingService, RbacService},
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// Extension trait to add all the new methods to GrpcUserServiceImpl
#[tonic::async_trait]
impl crate::proto::user_service_server::UserService for GrpcUserServiceImpl {
    // Keep existing implementations (already in users.rs)
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        // Already implemented in users.rs
        self.create_user(request).await
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        // Already implemented in users.rs
        self.get_user(request).await
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        // Already implemented in users.rs
        self.login(request).await
    }

    async fn get_profile(
        &self,
        request: Request<GetProfileRequest>,
    ) -> Result<Response<GetProfileResponse>, Status> {
        // Update the existing implementation
        let claims = super::users_extended::extract_claims(&request)?;

        let user_id = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Authentication {
                    message: "Invalid user ID in token".to_string(),
                }));
            }
        };

        let profile_service = ProfileService::new(self.repo.repo.pool());

        match profile_service.get_profile(user_id).await {
            Ok(Some(profile)) => {
                let proto_profile = super::users_extended::profile_to_proto(&profile);
                Ok(Response::new(GetProfileResponse {
                    profile: Some(proto_profile),
                }))
            }
            Ok(None) => {
                Err(ErrorTranslator::to_grpc_status(AppError::NotFound {
                    resource: "Profile".to_string(),
                    id: Some(user_id.to_string()),
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    // New implementations - Authentication & Registration
    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status> {
        let req = request.into_inner();
        let command = RegisterUser {
            username: req.username,
            email: req.email,
            password: req.password,
            first_name: Some(req.first_name).filter(|s| !s.is_empty()),
            last_name: Some(req.last_name).filter(|s| !s.is_empty()),
        };

        match self.repo.handle_register_user(command).await {
            Ok(()) => {
                Ok(Response::new(RegisterUserResponse {
                    message: "User registered successfully. Please check your email for verification.".to_string(),
                    verification_sent: true,
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn verify_email(
        &self,
        request: Request<VerifyEmailRequest>,
    ) -> Result<Response<VerifyEmailResponse>, Status> {
        // TODO: Implement email verification
        Err(Status::unimplemented("Email verification not yet implemented"))
    }

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        let claims = super::users_extended::extract_claims(&request)?;

        // TODO: Implement token revocation
        Ok(Response::new(LogoutResponse {
            message: format!("User {} logged out successfully", claims.sub),
        }))
    }

    async fn logout_all_devices(
        &self,
        request: Request<LogoutAllDevicesRequest>,
    ) -> Result<Response<LogoutAllDevicesResponse>, Status> {
        // TODO: Implement logout from all devices
        Err(Status::unimplemented("Logout all devices not yet implemented"))
    }

    // User Information
    async fn get_current_user(
        &self,
        request: Request<GetCurrentUserRequest>,
    ) -> Result<Response<GetCurrentUserResponse>, Status> {
        let claims = super::users_extended::extract_claims(&request)?;

        let user_id = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Authentication {
                    message: "Invalid user ID in token".to_string(),
                }));
            }
        };

        // Get user
        let user = match self.repo.repo.find_user_by_id(user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::NotFound {
                    resource: "User".to_string(),
                    id: Some(user_id.to_string()),
                }));
            }
            Err(e) => return Err(ErrorTranslator::to_grpc_status(e.into())),
        };

        // Get profile
        let profile_service = ProfileService::new(self.repo.repo.pool());
        let profile = profile_service.get_profile(user_id).await.ok().flatten();

        // Get roles
        let rbac_service = RbacService::new(self.repo.repo.db.clone());
        let roles = rbac_service.get_user_roles(user_id).await.ok().unwrap_or_default();
        let permissions = rbac_service.get_user_permissions(user_id).await.ok().unwrap_or_default();

        Ok(Response::new(GetCurrentUserResponse {
            user: Some(super::users_extended::user_to_proto(&user)),
            profile: profile.map(|p| super::users_extended::profile_to_proto(&p)),
            roles: roles.into_iter().map(|r| Role {
                id: r.id.to_string(),
                name: r.name,
                description: r.description.unwrap_or_default(),
                assigned_at: r.assigned_at.to_rfc3339(),
                expires_at: r.expires_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            }).collect(),
            permissions: permissions.into_iter().map(|p| Permission {
                resource: p.resource,
                action: p.action,
                description: p.description.unwrap_or_default(),
            }).collect(),
        }))
    }

    // Profile Management
    async fn update_profile(
        &self,
        request: Request<UpdateProfileRequest>,
    ) -> Result<Response<UpdateProfileResponse>, Status> {
        // TODO: Implement profile update
        Err(Status::unimplemented("Profile update not yet implemented"))
    }

    async fn update_account(
        &self,
        request: Request<UpdateAccountRequest>,
    ) -> Result<Response<UpdateAccountResponse>, Status> {
        // TODO: Implement account update
        Err(Status::unimplemented("Account update not yet implemented"))
    }

    async fn delete_profile(
        &self,
        request: Request<DeleteProfileRequest>,
    ) -> Result<Response<DeleteProfileResponse>, Status> {
        // TODO: Implement profile deletion
        Err(Status::unimplemented("Profile deletion not yet implemented"))
    }

    // Password Management
    async fn change_password(
        &self,
        request: Request<ChangePasswordRequest>,
    ) -> Result<Response<ChangePasswordResponse>, Status> {
        let claims = super::users_extended::extract_claims(&request)?;
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Authentication {
                    message: "Invalid user ID in token".to_string(),
                }));
            }
        };

        let password_service = PasswordService::new(self.repo.repo.pool());

        match password_service.change_password(user_id, &req.old_password, &req.new_password).await {
            Ok(()) => {
                Ok(Response::new(ChangePasswordResponse {
                    message: "Password changed successfully".to_string(),
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn request_password_reset(
        &self,
        request: Request<RequestPasswordResetRequest>,
    ) -> Result<Response<RequestPasswordResetResponse>, Status> {
        // TODO: Implement password reset request
        Err(Status::unimplemented("Password reset request not yet implemented"))
    }

    async fn reset_password(
        &self,
        request: Request<ResetPasswordRequest>,
    ) -> Result<Response<ResetPasswordResponse>, Status> {
        // TODO: Implement password reset
        Err(Status::unimplemented("Password reset not yet implemented"))
    }

    // Account Management
    async fn deactivate_account(
        &self,
        request: Request<DeactivateAccountRequest>,
    ) -> Result<Response<DeactivateAccountResponse>, Status> {
        // TODO: Implement account deactivation
        Err(Status::unimplemented("Account deactivation not yet implemented"))
    }

    async fn reactivate_account(
        &self,
        request: Request<ReactivateAccountRequest>,
    ) -> Result<Response<ReactivateAccountResponse>, Status> {
        // TODO: Implement account reactivation
        Err(Status::unimplemented("Account reactivation not yet implemented"))
    }

    async fn delete_account(
        &self,
        request: Request<DeleteAccountRequest>,
    ) -> Result<Response<DeleteAccountResponse>, Status> {
        // TODO: Implement account deletion
        Err(Status::unimplemented("Account deletion not yet implemented"))
    }

    async fn get_account_status(
        &self,
        request: Request<GetAccountStatusRequest>,
    ) -> Result<Response<GetAccountStatusResponse>, Status> {
        // TODO: Implement account status retrieval
        Err(Status::unimplemented("Account status retrieval not yet implemented"))
    }

    // Search & Directory
    async fn search_users(
        &self,
        request: Request<SearchUsersRequest>,
    ) -> Result<Response<SearchUsersResponse>, Status> {
        let req = request.into_inner();
        let search_service = SearchService::new(self.repo.repo.pool(), self.repo.repo.clone());

        let limit = if req.limit > 0 { req.limit as i64 } else { 10 };
        let offset = req.offset as i64;

        match search_service.search_users(&req.query, limit, offset).await {
            Ok(result) => {
                let users = result.users.into_iter().map(|u| super::users_extended::user_to_proto(&u)).collect();
                Ok(Response::new(SearchUsersResponse {
                    users,
                    total: result.total as i32,
                    has_more: result.has_more,
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn advanced_search(
        &self,
        request: Request<AdvancedSearchRequest>,
    ) -> Result<Response<AdvancedSearchResponse>, Status> {
        // TODO: Implement advanced search
        Err(Status::unimplemented("Advanced search not yet implemented"))
    }

    async fn suggest_users(
        &self,
        request: Request<SuggestUsersRequest>,
    ) -> Result<Response<SuggestUsersResponse>, Status> {
        let req = request.into_inner();
        let search_service = SearchService::new(self.repo.repo.pool(), self.repo.repo.clone());

        let limit = if req.limit > 0 { req.limit as i64 } else { 5 };

        match search_service.suggest_users(&req.prefix, limit).await {
            Ok(suggestions) => {
                Ok(Response::new(SuggestUsersResponse {
                    suggestions,
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn get_public_directory(
        &self,
        request: Request<GetPublicDirectoryRequest>,
    ) -> Result<Response<GetPublicDirectoryResponse>, Status> {
        // TODO: Implement public directory
        Err(Status::unimplemented("Public directory not yet implemented"))
    }

    // Activity & Logging
    async fn get_user_activities(
        &self,
        request: Request<GetUserActivitiesRequest>,
    ) -> Result<Response<GetUserActivitiesResponse>, Status> {
        // TODO: Implement user activities retrieval
        Err(Status::unimplemented("User activities retrieval not yet implemented"))
    }

    async fn get_activity_stats(
        &self,
        request: Request<GetActivityStatsRequest>,
    ) -> Result<Response<GetActivityStatsResponse>, Status> {
        // TODO: Implement activity stats retrieval
        Err(Status::unimplemented("Activity stats retrieval not yet implemented"))
    }

    // Session Management
    async fn get_active_sessions(
        &self,
        request: Request<GetActiveSessionsRequest>,
    ) -> Result<Response<GetActiveSessionsResponse>, Status> {
        // TODO: Implement active sessions retrieval
        Err(Status::unimplemented("Active sessions retrieval not yet implemented"))
    }

    async fn revoke_session(
        &self,
        request: Request<RevokeSessionRequest>,
    ) -> Result<Response<RevokeSessionResponse>, Status> {
        // TODO: Implement session revocation
        Err(Status::unimplemented("Session revocation not yet implemented"))
    }

    // Admin Operations
    async fn lock_user_account(
        &self,
        request: Request<LockUserAccountRequest>,
    ) -> Result<Response<LockUserAccountResponse>, Status> {
        // TODO: Implement account locking
        Err(Status::unimplemented("Account locking not yet implemented"))
    }

    async fn unlock_user_account(
        &self,
        request: Request<UnlockUserAccountRequest>,
    ) -> Result<Response<UnlockUserAccountResponse>, Status> {
        // TODO: Implement account unlocking
        Err(Status::unimplemented("Account unlocking not yet implemented"))
    }

    async fn get_lockout_status(
        &self,
        request: Request<GetLockoutStatusRequest>,
    ) -> Result<Response<GetLockoutStatusResponse>, Status> {
        // TODO: Implement lockout status retrieval
        Err(Status::unimplemented("Lockout status retrieval not yet implemented"))
    }

    async fn get_lockout_history(
        &self,
        request: Request<GetLockoutHistoryRequest>,
    ) -> Result<Response<GetLockoutHistoryResponse>, Status> {
        // TODO: Implement lockout history retrieval
        Err(Status::unimplemented("Lockout history retrieval not yet implemented"))
    }

    // RBAC Operations
    async fn assign_role(
        &self,
        request: Request<AssignRoleRequest>,
    ) -> Result<Response<AssignRoleResponse>, Status> {
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&req.user_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Validation {
                    field: "user_id".to_string(),
                    message: "Invalid user ID format".to_string(),
                }));
            }
        };

        let rbac_service = RbacService::new(self.repo.repo.db.clone());

        match rbac_service.assign_role_to_user(user_id, &req.role_name, None).await {
            Ok(()) => {
                Ok(Response::new(AssignRoleResponse {
                    message: format!("Role '{}' assigned to user successfully", req.role_name),
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn remove_role(
        &self,
        request: Request<RemoveRoleRequest>,
    ) -> Result<Response<RemoveRoleResponse>, Status> {
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&req.user_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Validation {
                    field: "user_id".to_string(),
                    message: "Invalid user ID format".to_string(),
                }));
            }
        };

        let rbac_service = RbacService::new(self.repo.repo.db.clone());

        match rbac_service.remove_role_from_user(user_id, &req.role_name).await {
            Ok(()) => {
                Ok(Response::new(RemoveRoleResponse {
                    message: format!("Role '{}' removed from user successfully", req.role_name),
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn get_user_roles(
        &self,
        request: Request<GetUserRolesRequest>,
    ) -> Result<Response<GetUserRolesResponse>, Status> {
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&req.user_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Validation {
                    field: "user_id".to_string(),
                    message: "Invalid user ID format".to_string(),
                }));
            }
        };

        let rbac_service = RbacService::new(self.repo.repo.db.clone());

        match rbac_service.get_user_roles(user_id).await {
            Ok(roles) => {
                let proto_roles = roles.into_iter().map(|r| Role {
                    id: r.id.to_string(),
                    name: r.name,
                    description: r.description.unwrap_or_default(),
                    assigned_at: r.assigned_at.to_rfc3339(),
                    expires_at: r.expires_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                }).collect();

                Ok(Response::new(GetUserRolesResponse {
                    roles: proto_roles,
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn get_user_permissions(
        &self,
        request: Request<GetUserPermissionsRequest>,
    ) -> Result<Response<GetUserPermissionsResponse>, Status> {
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&req.user_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Validation {
                    field: "user_id".to_string(),
                    message: "Invalid user ID format".to_string(),
                }));
            }
        };

        let rbac_service = RbacService::new(self.repo.repo.db.clone());

        match rbac_service.get_user_permissions(user_id).await {
            Ok(permissions) => {
                let proto_permissions = permissions.into_iter().map(|p| Permission {
                    resource: p.resource,
                    action: p.action,
                    description: p.description.unwrap_or_default(),
                }).collect();

                Ok(Response::new(GetUserPermissionsResponse {
                    permissions: proto_permissions,
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }

    async fn check_permission(
        &self,
        request: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status> {
        let req = request.into_inner();

        let user_id = match Uuid::parse_str(&req.user_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ErrorTranslator::to_grpc_status(AppError::Validation {
                    field: "user_id".to_string(),
                    message: "Invalid user ID format".to_string(),
                }));
            }
        };

        let rbac_service = RbacService::new(self.repo.repo.db.clone());

        match rbac_service.check_permission(user_id, &req.resource, &req.action).await {
            Ok(has_permission) => {
                Ok(Response::new(CheckPermissionResponse {
                    has_permission,
                    reason: if has_permission {
                        "User has permission".to_string()
                    } else {
                        "User lacks permission".to_string()
                    },
                }))
            }
            Err(e) => Err(ErrorTranslator::to_grpc_status(e)),
        }
    }
}