use axum::{
    Json,
    extract::{State, ConnectInfo},
    response::IntoResponse,
    http::HeaderMap,
};
use tracing::{error, info, warn};
use std::net::SocketAddr;

use crate::{
    commands,
    infrastructure::errors::ErrorTranslator,
    services::{UserService, RateLimitingService, ActivityLoggingService, AccountLockoutService},
};

/// Rate-limited login endpoint
pub async fn login_with_rate_limiting(
    State((user_service, db)): State<(UserService, crate::PostgreSQL)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<commands::Login>,
) -> impl IntoResponse {
    let ip_address = addr.ip().to_string();
    let user_agent = headers.get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let email = payload.email.clone();
    let rate_limit_service = RateLimitingService::new(db.pool());
    let activity_service = ActivityLoggingService::new(db.pool());

    // First check IP-based rate limiting (we don't know user_id yet)
    match rate_limit_service.check_ip_rate_limit(&ip_address).await {
        Ok(ip_limit) => {
            if ip_limit.is_limited {
                warn!("Login attempt blocked due to IP rate limit: {}", ip_address);

                // Log the blocked attempt
                let _ = activity_service.log_login(
                    uuid::Uuid::nil(), // Unknown user at this point
                    Some(ip_address.clone()),
                    user_agent.clone(),
                    false,
                ).await;

                return Json(serde_json::json!({
                    "error": "Too many failed login attempts",
                    "message": "Please try again later",
                    "rate_limit": {
                        "remaining_attempts": 0,
                        "reset_time": ip_limit.reset_time,
                        "window_minutes": ip_limit.window_minutes
                    }
                }))
                .into_response();
            }
        }
        Err(e) => {
            error!("Failed to check IP rate limit: {:?}", e);
            // Continue with login attempt despite rate limit check failure
        }
    }

    // Attempt to find user first to get user_id for user-specific rate limiting
    let user_id = match user_service.repo.find_user_by_email(&email).await {
        Ok(Some(user)) => Some(user.id),
        Ok(None) => None,
        Err(_) => None, // Continue with login attempt
    };

    // If we have a user_id, check account lockout status first
    if let Some(user_id) = user_id {
        let lockout_service = AccountLockoutService::new(db.pool());

        // Check if account is locked
        match lockout_service.is_account_locked(user_id).await {
            Ok(lockout_status) => {
                if lockout_status.is_locked {
                    warn!("Login attempt blocked due to account lockout: {} ({})", email, user_id);

                    // Record unlock attempt (user trying to access locked account)
                    let _ = lockout_service.record_unlock_attempt(user_id, Some(ip_address.clone())).await;

                    // Log the blocked attempt
                    let _ = activity_service.log_login(
                        user_id,
                        Some(ip_address),
                        user_agent,
                        false,
                    ).await;

                    let lockout_message = match lockout_status.lockout_type {
                        Some(crate::services::LockoutType::Temporary) => {
                            if let Some(locked_until) = lockout_status.locked_until {
                                format!("Account is temporarily locked until {}. Please try again later.",
                                       locked_until.format("%Y-%m-%d %H:%M:%S UTC"))
                            } else {
                                "Account is temporarily locked. Please try again later.".to_string()
                            }
                        }
                        Some(crate::services::LockoutType::Permanent) => {
                            "Account is permanently locked. Please contact support.".to_string()
                        }
                        None => "Account is locked. Please contact support.".to_string(),
                    };

                    return Json(serde_json::json!({
                        "error": "Account locked",
                        "message": lockout_message,
                        "lockout_status": lockout_status
                    }))
                    .into_response();
                }
            }
            Err(e) => {
                error!("Failed to check account lockout status: {:?}", e);
                // Continue with login attempt despite lockout check failure
            }
        }

        // Check user-specific rate limiting
        match rate_limit_service.check_user_rate_limit(user_id).await {
            Ok(user_limit) => {
                if user_limit.is_limited {
                    warn!("Login attempt blocked due to user rate limit: {} ({})", email, user_id);

                    // Record the failed attempt
                    let _ = rate_limit_service.record_login_attempt(
                        Some(ip_address.clone()),
                        Some(user_id),
                        Some(email.clone()),
                        user_agent.clone(),
                        false,
                    ).await;

                    // Log the blocked attempt
                    let _ = activity_service.log_login(
                        user_id,
                        Some(ip_address),
                        user_agent,
                        false,
                    ).await;

                    return Json(serde_json::json!({
                        "error": "Too many failed login attempts",
                        "message": "Account temporarily locked. Please try again later",
                        "rate_limit": {
                            "remaining_attempts": 0,
                            "reset_time": user_limit.reset_time,
                            "window_minutes": user_limit.window_minutes
                        }
                    }))
                    .into_response();
                }
            }
            Err(e) => {
                error!("Failed to check user rate limit: {:?}", e);
                // Continue with login attempt
            }
        }
    }

    // Proceed with actual login attempt
    match user_service.handle_login(payload).await {
        Ok(token) => {
            info!("Login successful for user: {} from IP: {}", email, ip_address);

            // Record successful login attempt
            if let Err(e) = rate_limit_service.record_login_attempt(
                Some(ip_address.clone()),
                user_id,
                Some(email.clone()),
                user_agent.clone(),
                true, // success = true
            ).await {
                error!("Failed to record successful login attempt: {:?}", e);
            }

            // Log successful login activity
            if let Some(user_id) = user_id {
                if let Err(e) = activity_service.log_login(
                    user_id,
                    Some(ip_address),
                    user_agent,
                    true,
                ).await {
                    error!("Failed to log successful login activity: {:?}", e);
                }
            }

            Json(serde_json::json!({
                "token": token,
                "message": "Login successful"
            })).into_response()
        }
        Err(app_error) => {
            warn!("Login failed for user {} from IP {}: {:?}", email, ip_address, app_error);

            // Record failed login attempt
            if let Err(e) = rate_limit_service.record_login_attempt(
                Some(ip_address.clone()),
                user_id,
                Some(email.clone()),
                user_agent.clone(),
                false, // success = false
            ).await {
                error!("Failed to record failed login attempt: {:?}", e);
            }

            // Log failed login activity
            if let Some(user_id) = user_id {
                if let Err(e) = activity_service.log_login(
                    user_id,
                    Some(ip_address.clone()),
                    user_agent.clone(),
                    false,
                ).await {
                    error!("Failed to log failed login activity: {:?}", e);
                }

                // Check if we should trigger account lockout based on failed attempts
                if let Ok(user_stats) = rate_limit_service.get_user_stats(user_id).await {
                    let total_failed_attempts = user_stats.failed_attempts_24h;

                    // Trigger lockout if failed attempts exceed thresholds
                    if total_failed_attempts >= 5 {
                        let lockout_service = AccountLockoutService::new(db.pool());

                        // Check if user is already locked to avoid duplicate lockouts
                        match lockout_service.is_account_locked(user_id).await {
                            Ok(lockout_status) => {
                                if !lockout_status.is_locked {
                                    // Create new lockout based on failed attempts
                                    match lockout_service.lock_account_for_failed_attempts(user_id, total_failed_attempts).await {
                                        Ok(lockout_info) => {
                                            warn!("Account {} locked after {} failed attempts: {:?}",
                                                 user_id, total_failed_attempts, lockout_info);
                                        }
                                        Err(e) => {
                                            error!("Failed to lock account {} after {} failed attempts: {:?}",
                                                  user_id, total_failed_attempts, e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to check lockout status before creating lockout: {:?}", e);
                            }
                        }
                    }
                }
            }

            // Check current rate limit status to include in response
            let rate_limit_info = if let Ok(combined_limit) = rate_limit_service.check_combined_rate_limit(&ip_address, user_id).await {
                Some(serde_json::json!({
                    "remaining_attempts": combined_limit.remaining_attempts,
                    "reset_time": combined_limit.reset_time,
                    "window_minutes": combined_limit.window_minutes
                }))
            } else {
                None
            };

            let mut response = serde_json::json!({
                "error": "Authentication failed",
                "message": "Invalid email or password"
            });

            if let Some(rate_limit) = rate_limit_info {
                response["rate_limit"] = rate_limit;
            }

            // Use original error translation for consistency
            ErrorTranslator::to_http_response(app_error)
        }
    }
}

/// Get current rate limit status for the requesting IP
pub async fn get_rate_limit_status(
    State((_, db)): State<(UserService, crate::PostgreSQL)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip_address = addr.ip().to_string();
    let rate_limit_service = RateLimitingService::new(db.pool());

    match rate_limit_service.check_ip_rate_limit(&ip_address).await {
        Ok(rate_limit_result) => {
            Json(serde_json::json!({
                "success": true,
                "ip_address": ip_address,
                "rate_limit": rate_limit_result
            }))
            .into_response()
        }
        Err(app_error) => {
            error!("Failed to check rate limit status for IP {}: {:?}", ip_address, app_error);
            ErrorTranslator::to_http_response(app_error)
        }
    }
}