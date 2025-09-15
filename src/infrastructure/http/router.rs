use axum::{
    Router, middleware,
    routing::{Router as HttpRouter, get, post, put, delete},
};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;

use crate::{Api, PostgreSQL, commands::CommandMessage, services::UserService};

use super::{
    controllers::{create_user, get_profile, get_user_by_id, login, register_user, verify_email, change_password,
                  request_password_reset, reset_password,
                  get_user_profile, update_user_profile, update_user_account, delete_user_profile,
                  deactivate_account, reactivate_account, delete_account, get_account_status,
                  search_users, advanced_search, suggest_users, public_user_directory, search_statistics,
                  get_user_activities, get_user_activity_stats, cleanup_old_activities,
                  get_rate_limit_stats, get_top_failing_ips, cleanup_rate_limit_records, check_ip_rate_limit,
                  login_with_rate_limiting, get_rate_limit_status,
                  get_lockout_status, get_user_lockout_history, lock_user_account, unlock_user_account,
                  get_lockout_statistics, get_admin_lockout_history, cleanup_expired_lockouts,
                  logout, logout_all_devices, get_active_sessions, revoke_session, cleanup_sessions,
                  get_users_paginated, get_pagination_demo, get_pagination_comparison,
                  bulk_create_users, bulk_update_user_status, bulk_delete_users,
                  get_bulk_operation_status, get_bulk_capabilities,
                  get_api_documentation, get_api_endpoints, get_api_schemas,
                  get_auth_documentation, get_api_examples, get_postman_collection,
                  versioned},
    middleware::auth_middleware,
    versioning::{versioning_middleware, api_info, unsupported_version_handler},
};

pub fn router(pool: Pool<Postgres>, sender: mpsc::Sender<CommandMessage>) -> HttpRouter {
    let postgres_db = PostgreSQL::new(pool.clone());
    let user_service = UserService::new(postgres_db.clone(), sender);

    let public_routes = Router::new()
        .route(Api::CreateUser.into(), post(create_user))
        .route(Api::GetUser.into(), get(get_user_by_id))
        .route(Api::Login.into(), post(login))
        .route("/api/register", post(register_user))
        .route("/api/verify-email", post(verify_email))
        .with_state(user_service.clone());

    let public_search_routes = Router::new()
        .route("/api/users/directory", get(public_user_directory))
        .with_state(postgres_db.clone());

    let pagination_routes = Router::new()
        .route("/api/users/paginated", get(get_users_paginated))
        .route("/api/pagination/demo", get(get_pagination_demo))
        .route("/api/pagination/comparison", get(get_pagination_comparison))
        .with_state(postgres_db.clone());

    let auth_routes = Router::new()
        .route("/api/auth/login", post(login_with_rate_limiting))
        .route("/api/auth/rate-limit-status", get(get_rate_limit_status))
        .with_state((user_service.clone(), postgres_db.clone()));

    let account_routes = Router::new()
        .route("/api/account/reactivate", post(reactivate_account))
        .with_state(postgres_db.clone());

    let password_reset_routes = Router::new()
        .route("/api/request-password-reset", post(request_password_reset))
        .route("/api/reset-password", post(reset_password))
        .with_state(postgres_db.clone());

    let protected_routes = Router::new()
        .route(Api::Profile.into(), get(get_profile))
        .route("/api/change-password", post(change_password))
        .route("/api/user/profile", get(get_user_profile))
        .route("/api/user/profile", put(update_user_profile))
        .route("/api/user/profile", delete(delete_user_profile))
        .route("/api/user/account", put(update_user_account))
        .route("/api/account/deactivate", post(deactivate_account))
        .route("/api/account/delete", delete(delete_account))
        .route("/api/account/status", get(get_account_status))
        .route("/api/users/search", get(search_users))
        .route("/api/users/search/advanced", post(advanced_search))
        .route("/api/users/suggest", get(suggest_users))
        .route("/api/users/statistics", get(search_statistics))
        .route("/api/user/activities", get(get_user_activities))
        .route("/api/user/activities/stats", get(get_user_activity_stats))
        .route("/api/admin/activities/cleanup", delete(cleanup_old_activities))
        .route("/api/admin/rate-limits/stats", get(get_rate_limit_stats))
        .route("/api/admin/rate-limits/failing-ips", get(get_top_failing_ips))
        .route("/api/admin/rate-limits/cleanup", delete(cleanup_rate_limit_records))
        .route("/api/admin/rate-limits/check-ip", get(check_ip_rate_limit))
        .route("/api/user/lockout/status", get(get_lockout_status))
        .route("/api/user/lockout/history", get(get_user_lockout_history))
        .route("/api/admin/users/{user_id}/lock", post(lock_user_account))
        .route("/api/admin/users/{user_id}/unlock", post(unlock_user_account))
        .route("/api/admin/lockout/statistics", get(get_lockout_statistics))
        .route("/api/admin/users/{user_id}/lockout/history", get(get_admin_lockout_history))
        .route("/api/admin/lockout/cleanup", delete(cleanup_expired_lockouts))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/logout-all", post(logout_all_devices))
        .route("/api/auth/sessions", get(get_active_sessions))
        .route("/api/auth/sessions/{session_id}", delete(revoke_session))
        .route("/api/admin/sessions/cleanup", delete(cleanup_sessions))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(postgres_db.clone());

    // Bulk operations routes (admin-only)
    let bulk_operations_routes = Router::new()
        .route("/api/bulk/users/create", post(bulk_create_users))
        .route("/api/bulk/users/status", put(bulk_update_user_status))
        .route("/api/bulk/users/delete", delete(bulk_delete_users))
        .route("/api/bulk/operations/{operation_id}/status", get(get_bulk_operation_status))
        .route("/api/bulk/capabilities", get(get_bulk_capabilities))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(postgres_db.clone());

    // API documentation routes (public)
    let documentation_routes = Router::new()
        .route("/api/docs", get(get_api_documentation))
        .route("/api/docs/openapi.json", get(get_api_documentation))
        .route("/api/docs/endpoints", get(get_api_endpoints))
        .route("/api/docs/schemas", get(get_api_schemas))
        .route("/api/docs/auth", get(get_auth_documentation))
        .route("/api/docs/examples", get(get_api_examples))
        .route("/api/docs/postman-collection.json", get(get_postman_collection))
        .with_state(postgres_db.clone());

    // Versioned API routes
    let versioned_routes = Router::new()
        // API info endpoints
        .route("/api", get(|| api_info(None)))
        .route("/api/", get(|| api_info(None)))
        .route("/api/{version}", get(|path| api_info(Some(path))))
        .route("/api/{version}/", get(|path| api_info(Some(path))))

        // V1 routes (deprecated)
        .route("/api/v1/profile", get(versioned::user_v1::get_profile_v1))
        .route("/api/v1/register", post(versioned::user_v1::register_v1))
        .route("/api/v1/users/{user_id}", get(versioned::user_v1::get_user_v1))
        .route("/api/v1/login", post(versioned::user_v1::login_v1))

        // V2 routes (stable)
        .route("/api/v2/profile", get(versioned::user_v2::get_profile_v2))
        .route("/api/v2/register", post(versioned::user_v2::register_v2))
        .route("/api/v2/users/{user_id}", get(versioned::user_v2::get_user_v2))
        .route("/api/v2/login", post(versioned::user_v2::login_v2))

        // V3 routes (latest)
        .route("/api/v3/profile", get(versioned::user_v3::get_profile_v3))
        .route("/api/v3/register", post(versioned::user_v3::register_v3))
        .route("/api/v3/users/{user_id}", get(versioned::user_v3::get_user_v3))
        .route("/api/v3/login", post(versioned::user_v3::login_v3))

        // Fallback for unsupported versions
        .fallback(unsupported_version_handler)

        .layer(middleware::from_fn(versioning_middleware))
        .with_state(user_service);

    Router::new()
        .merge(public_routes)
        .merge(public_search_routes)
        .merge(pagination_routes)
        .merge(auth_routes)
        .merge(account_routes)
        .merge(password_reset_routes)
        .merge(protected_routes)
        .merge(bulk_operations_routes)
        .merge(documentation_routes)
        .merge(versioned_routes)
}
