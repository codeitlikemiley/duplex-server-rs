//! Extended gRPC User Service implementation for new operations
//!
//! This module extends the base user service with all the new operations
//! for comprehensive user management.

use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tracing;
use uuid::Uuid;
use chrono::Utc;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, RegisterUser, VerifyEmail, ChangePassword, send_command},
    errors::AppError,
    infrastructure::{
        errors::ErrorTranslator,
        auth::{JwtService, Claims},
    },
    proto::{
        // Authentication & Registration
        RegisterUserRequest, RegisterUserResponse,
        VerifyEmailRequest, VerifyEmailResponse,
        LogoutRequest, LogoutResponse,
        LogoutAllDevicesRequest, LogoutAllDevicesResponse,

        // User Information
        GetCurrentUserRequest, GetCurrentUserResponse,

        // Profile Management
        UpdateProfileRequest, UpdateProfileResponse,
        UpdateAccountRequest, UpdateAccountResponse,
        DeleteProfileRequest, DeleteProfileResponse,

        // Password Management
        ChangePasswordRequest, ChangePasswordResponse,
        RequestPasswordResetRequest, RequestPasswordResetResponse,
        ResetPasswordRequest, ResetPasswordResponse,

        // Account Management
        DeactivateAccountRequest, DeactivateAccountResponse,
        ReactivateAccountRequest, ReactivateAccountResponse,
        DeleteAccountRequest, DeleteAccountResponse,
        GetAccountStatusRequest, GetAccountStatusResponse,

        // Search
        SearchUsersRequest, SearchUsersResponse,
        AdvancedSearchRequest, AdvancedSearchResponse,
        SuggestUsersRequest, SuggestUsersResponse,
        GetPublicDirectoryRequest, GetPublicDirectoryResponse,

        // Activity
        GetUserActivitiesRequest, GetUserActivitiesResponse,
        GetActivityStatsRequest, GetActivityStatsResponse,

        // Sessions
        GetActiveSessionsRequest, GetActiveSessionsResponse,
        RevokeSessionRequest, RevokeSessionResponse,

        // Admin Operations
        LockUserAccountRequest, LockUserAccountResponse,
        UnlockUserAccountRequest, UnlockUserAccountResponse,
        GetLockoutStatusRequest, GetLockoutStatusResponse,
        GetLockoutHistoryRequest, GetLockoutHistoryResponse,

        // RBAC
        AssignRoleRequest, AssignRoleResponse,
        RemoveRoleRequest, RemoveRoleResponse,
        GetUserRolesRequest, GetUserRolesResponse,
        GetUserPermissionsRequest, GetUserPermissionsResponse,
        CheckPermissionRequest, CheckPermissionResponse,

        // Common messages
        User as ProtoUser, UserProfile as ProtoUserProfile,
        Role as ProtoRole, Permission as ProtoPermission,
        Activity as ProtoActivity, Session as ProtoSession,
        UserStatus as ProtoUserStatus, ActivityType as ProtoActivityType,
    },
    services::{
        UserService, ProfileService, PasswordService, PasswordResetService,
        SearchService, ActivityLoggingService, LogoutService, LockoutService,
        RbacService,
    },
    models::{User, UserProfile, UserStatus},
};

/// Helper to extract claims from gRPC request metadata
pub fn extract_claims(request: &Request<impl std::fmt::Debug>) -> Result<Claims, Status> {
    let jwt_service = JwtService::default();

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
                message: "Missing or invalid authorization header".to_string(),
            };
            return Err(ErrorTranslator::to_grpc_status(error));
        }
    };

    // Verify token
    match jwt_service.verify_token(token) {
        Ok(claims) => Ok(claims),
        Err(_) => {
            let error = AppError::Authentication {
                message: "Invalid or expired JWT token".to_string(),
            };
            Err(ErrorTranslator::to_grpc_status(error))
        }
    }
}

/// Convert domain User to proto User
pub fn user_to_proto(user: &User) -> ProtoUser {
    ProtoUser {
        id: user.id.to_string(),
        username: user.username.clone(),
        email: user.email.clone(),
        email_verified: user.email_verified,
        status: match user.status {
            UserStatus::Active => ProtoUserStatus::UserStatusActive as i32,
            UserStatus::Inactive => ProtoUserStatus::UserStatusInactive as i32,
            UserStatus::Suspended => ProtoUserStatus::UserStatusSuspended as i32,
            UserStatus::PendingVerification => ProtoUserStatus::UserStatusPendingVerification as i32,
        },
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
        last_login_at: user.last_login_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
    }
}

/// Convert domain UserProfile to proto UserProfile
pub fn profile_to_proto(profile: &UserProfile) -> ProtoUserProfile {
    ProtoUserProfile {
        user_id: profile.user_id.to_string(),
        first_name: profile.first_name.clone().unwrap_or_default(),
        last_name: profile.last_name.clone().unwrap_or_default(),
        bio: profile.bio.clone().unwrap_or_default(),
        location: profile.location.clone().unwrap_or_default(),
        website: profile.website.clone().unwrap_or_default(),
        avatar_url: profile.avatar_url.clone().unwrap_or_default(),
        preferences_json: profile.preferences.to_string(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

/// Extended implementation trait for new user service operations
#[tonic::async_trait]
pub trait ExtendedUserService {
    // Authentication & Registration
    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status>;

    async fn verify_email(
        &self,
        request: Request<VerifyEmailRequest>,
    ) -> Result<Response<VerifyEmailResponse>, Status>;

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status>;

    async fn logout_all_devices(
        &self,
        request: Request<LogoutAllDevicesRequest>,
    ) -> Result<Response<LogoutAllDevicesResponse>, Status>;

    // User Information
    async fn get_current_user(
        &self,
        request: Request<GetCurrentUserRequest>,
    ) -> Result<Response<GetCurrentUserResponse>, Status>;

    // Profile Management
    async fn update_profile(
        &self,
        request: Request<UpdateProfileRequest>,
    ) -> Result<Response<UpdateProfileResponse>, Status>;

    async fn update_account(
        &self,
        request: Request<UpdateAccountRequest>,
    ) -> Result<Response<UpdateAccountResponse>, Status>;

    async fn delete_profile(
        &self,
        request: Request<DeleteProfileRequest>,
    ) -> Result<Response<DeleteProfileResponse>, Status>;

    // Password Management
    async fn change_password(
        &self,
        request: Request<ChangePasswordRequest>,
    ) -> Result<Response<ChangePasswordResponse>, Status>;

    async fn request_password_reset(
        &self,
        request: Request<RequestPasswordResetRequest>,
    ) -> Result<Response<RequestPasswordResetResponse>, Status>;

    async fn reset_password(
        &self,
        request: Request<ResetPasswordRequest>,
    ) -> Result<Response<ResetPasswordResponse>, Status>;

    // Account Management
    async fn deactivate_account(
        &self,
        request: Request<DeactivateAccountRequest>,
    ) -> Result<Response<DeactivateAccountResponse>, Status>;

    async fn reactivate_account(
        &self,
        request: Request<ReactivateAccountRequest>,
    ) -> Result<Response<ReactivateAccountResponse>, Status>;

    async fn delete_account(
        &self,
        request: Request<DeleteAccountRequest>,
    ) -> Result<Response<DeleteAccountResponse>, Status>;

    async fn get_account_status(
        &self,
        request: Request<GetAccountStatusRequest>,
    ) -> Result<Response<GetAccountStatusResponse>, Status>;

    // Search & Directory
    async fn search_users(
        &self,
        request: Request<SearchUsersRequest>,
    ) -> Result<Response<SearchUsersResponse>, Status>;

    async fn advanced_search(
        &self,
        request: Request<AdvancedSearchRequest>,
    ) -> Result<Response<AdvancedSearchResponse>, Status>;

    async fn suggest_users(
        &self,
        request: Request<SuggestUsersRequest>,
    ) -> Result<Response<SuggestUsersResponse>, Status>;

    async fn get_public_directory(
        &self,
        request: Request<GetPublicDirectoryRequest>,
    ) -> Result<Response<GetPublicDirectoryResponse>, Status>;

    // Activity & Logging
    async fn get_user_activities(
        &self,
        request: Request<GetUserActivitiesRequest>,
    ) -> Result<Response<GetUserActivitiesResponse>, Status>;

    async fn get_activity_stats(
        &self,
        request: Request<GetActivityStatsRequest>,
    ) -> Result<Response<GetActivityStatsResponse>, Status>;

    // Session Management
    async fn get_active_sessions(
        &self,
        request: Request<GetActiveSessionsRequest>,
    ) -> Result<Response<GetActiveSessionsResponse>, Status>;

    async fn revoke_session(
        &self,
        request: Request<RevokeSessionRequest>,
    ) -> Result<Response<RevokeSessionResponse>, Status>;

    // Admin Operations
    async fn lock_user_account(
        &self,
        request: Request<LockUserAccountRequest>,
    ) -> Result<Response<LockUserAccountResponse>, Status>;

    async fn unlock_user_account(
        &self,
        request: Request<UnlockUserAccountRequest>,
    ) -> Result<Response<UnlockUserAccountResponse>, Status>;

    async fn get_lockout_status(
        &self,
        request: Request<GetLockoutStatusRequest>,
    ) -> Result<Response<GetLockoutStatusResponse>, Status>;

    async fn get_lockout_history(
        &self,
        request: Request<GetLockoutHistoryRequest>,
    ) -> Result<Response<GetLockoutHistoryResponse>, Status>;

    // RBAC Operations
    async fn assign_role(
        &self,
        request: Request<AssignRoleRequest>,
    ) -> Result<Response<AssignRoleResponse>, Status>;

    async fn remove_role(
        &self,
        request: Request<RemoveRoleRequest>,
    ) -> Result<Response<RemoveRoleResponse>, Status>;

    async fn get_user_roles(
        &self,
        request: Request<GetUserRolesRequest>,
    ) -> Result<Response<GetUserRolesResponse>, Status>;

    async fn get_user_permissions(
        &self,
        request: Request<GetUserPermissionsRequest>,
    ) -> Result<Response<GetUserPermissionsResponse>, Status>;

    async fn check_permission(
        &self,
        request: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status>;
}