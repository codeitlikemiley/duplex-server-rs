use chrono::{DateTime, Utc, Duration};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::{
    errors::AppError,
};

pub struct RateLimitingService {
    db: Pool<Postgres>,
}

impl RateLimitingService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { db: pool }
    }

    /// Check if an IP address is rate limited for authentication attempts
    pub async fn check_ip_rate_limit(&self, ip_address: &str) -> Result<RateLimitResult, AppError> {
        let window_minutes = 15; // 15-minute sliding window
        let max_attempts = 10; // Max 10 attempts per window

        let window_start = Utc::now() - Duration::minutes(window_minutes);

        let attempt_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM auth_rate_limits WHERE ip_address = $1 AND created_at > $2",
            ip_address,
            window_start
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to check IP rate limit: {}", e)
        })?
        .unwrap_or(0);

        let is_limited = attempt_count >= max_attempts;
        let remaining_attempts = if is_limited { 0 } else { max_attempts - attempt_count };

        // Get the time when the rate limit will reset
        let reset_time = if is_limited {
            // Find the earliest attempt in the current window and add the window duration
            let earliest_attempt = sqlx::query_scalar!(
                "SELECT created_at FROM auth_rate_limits WHERE ip_address = $1 AND created_at > $2 ORDER BY created_at LIMIT 1",
                ip_address,
                window_start
            )
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to get earliest attempt: {}", e)
            })?;

            if let Some(earliest) = earliest_attempt {
                Some(earliest + Duration::minutes(window_minutes))
            } else {
                None
            }
        } else {
            None
        };

        Ok(RateLimitResult {
            is_limited,
            remaining_attempts: remaining_attempts as u32,
            reset_time,
            window_minutes: window_minutes as u32,
        })
    }

    /// Check if a user account is rate limited for authentication attempts
    pub async fn check_user_rate_limit(&self, user_id: Uuid) -> Result<RateLimitResult, AppError> {
        let window_minutes = 30; // 30-minute sliding window for user accounts
        let max_attempts = 5; // Max 5 failed attempts per user

        let window_start = Utc::now() - Duration::minutes(window_minutes);

        let attempt_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM auth_rate_limits WHERE user_id = $1 AND created_at > $2 AND success = false",
            user_id,
            window_start
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to check user rate limit: {}", e)
        })?
        .unwrap_or(0);

        let is_limited = attempt_count >= max_attempts;
        let remaining_attempts = if is_limited { 0 } else { max_attempts - attempt_count };

        let reset_time = if is_limited {
            let earliest_attempt = sqlx::query_scalar!(
                "SELECT created_at FROM auth_rate_limits WHERE user_id = $1 AND created_at > $2 AND success = false ORDER BY created_at LIMIT 1",
                user_id,
                window_start
            )
            .fetch_optional(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to get earliest user attempt: {}", e)
            })?;

            if let Some(earliest) = earliest_attempt {
                Some(earliest + Duration::minutes(window_minutes))
            } else {
                None
            }
        } else {
            None
        };

        Ok(RateLimitResult {
            is_limited,
            remaining_attempts: remaining_attempts as u32,
            reset_time,
            window_minutes: window_minutes as u32,
        })
    }

    /// Record an authentication attempt
    pub async fn record_auth_attempt(
        &self,
        attempt: AuthAttempt,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO auth_rate_limits (id, ip_address, user_id, username, success, user_agent, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            attempt.id,
            attempt.ip_address,
            attempt.user_id,
            attempt.username,
            attempt.success,
            attempt.user_agent,
            attempt.created_at
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to record auth attempt: {}", e)
        })?;

        Ok(())
    }

    /// Check both IP and user rate limits and return the most restrictive
    pub async fn check_combined_rate_limit(
        &self,
        ip_address: &str,
        user_id: Option<Uuid>,
    ) -> Result<RateLimitResult, AppError> {
        let ip_limit = self.check_ip_rate_limit(ip_address).await?;

        if let Some(user_id) = user_id {
            let user_limit = self.check_user_rate_limit(user_id).await?;

            // Return the most restrictive limit
            if ip_limit.is_limited || user_limit.is_limited {
                return Ok(RateLimitResult {
                    is_limited: true,
                    remaining_attempts: 0,
                    reset_time: match (ip_limit.reset_time, user_limit.reset_time) {
                        (Some(ip_reset), Some(user_reset)) => Some(ip_reset.max(user_reset)),
                        (Some(reset), None) | (None, Some(reset)) => Some(reset),
                        _ => None,
                    },
                    window_minutes: ip_limit.window_minutes.max(user_limit.window_minutes),
                });
            }

            // Return the most restrictive remaining attempts
            Ok(RateLimitResult {
                is_limited: false,
                remaining_attempts: ip_limit.remaining_attempts.min(user_limit.remaining_attempts),
                reset_time: None,
                window_minutes: ip_limit.window_minutes,
            })
        } else {
            Ok(ip_limit)
        }
    }

    /// Clean up old rate limiting records (for maintenance)
    pub async fn cleanup_old_records(&self, older_than_hours: i32) -> Result<u64, AppError> {
        let cutoff_time = Utc::now() - Duration::hours(older_than_hours as i64);

        let deleted_count = sqlx::query!(
            "DELETE FROM auth_rate_limits WHERE created_at < $1",
            cutoff_time
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup old rate limit records: {}", e)
        })?
        .rows_affected();

        Ok(deleted_count)
    }

    /// Get rate limiting statistics for monitoring
    pub async fn get_rate_limit_stats(&self) -> Result<RateLimitStats, AppError> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_attempts,
                COUNT(CASE WHEN success = true THEN 1 END) as successful_attempts,
                COUNT(CASE WHEN success = false THEN 1 END) as failed_attempts,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '1 hour' THEN 1 END) as attempts_last_hour,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '1 day' THEN 1 END) as attempts_last_day,
                COUNT(DISTINCT ip_address) as unique_ips,
                COUNT(DISTINCT user_id) as unique_users
            FROM auth_rate_limits
            WHERE created_at >= NOW() - INTERVAL '7 days'
            "#
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get rate limit statistics: {}", e)
        })?;

        Ok(RateLimitStats {
            total_attempts: stats.total_attempts.unwrap_or(0),
            successful_attempts: stats.successful_attempts.unwrap_or(0),
            failed_attempts: stats.failed_attempts.unwrap_or(0),
            attempts_last_hour: stats.attempts_last_hour.unwrap_or(0),
            attempts_last_day: stats.attempts_last_day.unwrap_or(0),
            unique_ips: stats.unique_ips.unwrap_or(0),
            unique_users: stats.unique_users.unwrap_or(0),
        })
    }

    /// Get top IP addresses with most failed attempts (for security monitoring)
    pub async fn get_top_failing_ips(&self, limit: i32) -> Result<Vec<FailingIpInfo>, AppError> {
        let failing_ips = sqlx::query_as!(
            FailingIpInfo,
            r#"
            SELECT
                ip_address,
                COUNT(*) as attempt_count,
                COUNT(CASE WHEN success = false THEN 1 END) as failed_count,
                MAX(created_at) as last_attempt_at
            FROM auth_rate_limits
            WHERE created_at >= NOW() - INTERVAL '24 hours'
            GROUP BY ip_address
            HAVING COUNT(CASE WHEN success = false THEN 1 END) >= 5
            ORDER BY failed_count DESC, attempt_count DESC
            LIMIT $1
            "#,
            limit as i64
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get top failing IPs: {}", e)
        })?;

        Ok(failing_ips)
    }
}

// Helper functions for creating auth attempts
impl RateLimitingService {
    pub async fn record_login_attempt(
        &self,
        ip_address: Option<String>,
        user_id: Option<Uuid>,
        username: Option<String>,
        user_agent: Option<String>,
        success: bool,
    ) -> Result<(), AppError> {
        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address,
            user_id,
            username,
            success,
            user_agent,
            created_at: Utc::now(),
        };

        self.record_auth_attempt(attempt).await
    }

    /// Check if authentication should be allowed
    pub async fn should_allow_auth(
        &self,
        ip_address: &str,
        user_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let limit_result = self.check_combined_rate_limit(ip_address, user_id).await?;
        Ok(!limit_result.is_limited)
    }

    /// Get user-specific statistics for the last 24 hours
    pub async fn get_user_stats(&self, user_id: Uuid) -> Result<UserStats, AppError> {
        let window_start = Utc::now() - Duration::hours(24);

        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(CASE WHEN success = false THEN 1 END) as failed_attempts,
                COUNT(CASE WHEN success = true THEN 1 END) as successful_attempts,
                MAX(created_at) as last_attempt_at
            FROM auth_rate_limits
            WHERE user_id = $1 AND created_at > $2
            "#,
            user_id,
            window_start
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get user stats: {}", e)
        })?;

        Ok(UserStats {
            failed_attempts_24h: stats.failed_attempts.unwrap_or(0) as i32,
            successful_attempts_24h: stats.successful_attempts.unwrap_or(0) as i32,
            last_attempt_at: stats.last_attempt_at,
        })
    }
}

// DTOs and Types
#[derive(Debug, Clone)]
pub struct AuthAttempt {
    pub id: Uuid,
    pub ip_address: Option<String>,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub success: bool,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitResult {
    pub is_limited: bool,
    pub remaining_attempts: u32,
    pub reset_time: Option<DateTime<Utc>>,
    pub window_minutes: u32,
}

#[derive(Debug, Serialize)]
pub struct RateLimitStats {
    pub total_attempts: i64,
    pub successful_attempts: i64,
    pub failed_attempts: i64,
    pub attempts_last_hour: i64,
    pub attempts_last_day: i64,
    pub unique_ips: i64,
    pub unique_users: i64,
}

#[derive(Debug, Serialize)]
pub struct FailingIpInfo {
    pub ip_address: Option<String>,
    pub attempt_count: Option<i64>,
    pub failed_count: Option<i64>,
    pub last_attempt_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitQuery {
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub failed_attempts_24h: i32,
    pub successful_attempts_24h: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
}