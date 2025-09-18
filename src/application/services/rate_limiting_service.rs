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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_attempt_creation() {
        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("192.168.1.1".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("testuser".to_string()),
            success: true,
            user_agent: Some("Mozilla/5.0".to_string()),
            created_at: Utc::now(),
        };

        assert!(attempt.ip_address.is_some());
        assert!(attempt.user_id.is_some());
        assert!(attempt.username.is_some());
        assert!(attempt.success);
        assert!(attempt.user_agent.is_some());
    }

    #[test]
    fn test_auth_attempt_failed() {
        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("10.0.0.1".to_string()),
            user_id: None,
            username: Some("attacker".to_string()),
            success: false,
            user_agent: None,
            created_at: Utc::now(),
        };

        assert!(!attempt.success);
        assert!(attempt.user_id.is_none());
        assert!(attempt.user_agent.is_none());
    }

    #[test]
    fn test_auth_attempt_clone() {
        let original = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("127.0.0.1".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("user".to_string()),
            success: true,
            user_agent: Some("curl/7.68.0".to_string()),
            created_at: Utc::now(),
        };

        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.ip_address, original.ip_address);
        assert_eq!(cloned.user_id, original.user_id);
        assert_eq!(cloned.username, original.username);
        assert_eq!(cloned.success, original.success);
        assert_eq!(cloned.user_agent, original.user_agent);
        assert_eq!(cloned.created_at, original.created_at);
    }

    #[test]
    fn test_rate_limit_result_not_limited() {
        let result = RateLimitResult {
            is_limited: false,
            remaining_attempts: 8,
            reset_time: None,
            window_minutes: 15,
        };

        assert!(!result.is_limited);
        assert_eq!(result.remaining_attempts, 8);
        assert!(result.reset_time.is_none());
        assert_eq!(result.window_minutes, 15);
    }

    #[test]
    fn test_rate_limit_result_limited() {
        let reset_time = Utc::now() + Duration::minutes(10);
        let result = RateLimitResult {
            is_limited: true,
            remaining_attempts: 0,
            reset_time: Some(reset_time),
            window_minutes: 15,
        };

        assert!(result.is_limited);
        assert_eq!(result.remaining_attempts, 0);
        assert_eq!(result.reset_time, Some(reset_time));
        assert_eq!(result.window_minutes, 15);
    }

    #[test]
    fn test_rate_limit_result_serialization() {
        let result = RateLimitResult {
            is_limited: true,
            remaining_attempts: 3,
            reset_time: Some(Utc::now()),
            window_minutes: 30,
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize RateLimitResult");
        assert!(json.contains("is_limited"));
        assert!(json.contains("remaining_attempts"));
        assert!(json.contains("reset_time"));
        assert!(json.contains("window_minutes"));
    }

    #[test]
    fn test_rate_limit_stats_creation() {
        let stats = RateLimitStats {
            total_attempts: 100,
            successful_attempts: 85,
            failed_attempts: 15,
            attempts_last_hour: 25,
            attempts_last_day: 100,
            unique_ips: 45,
            unique_users: 30,
        };

        assert_eq!(stats.total_attempts, 100);
        assert_eq!(stats.successful_attempts, 85);
        assert_eq!(stats.failed_attempts, 15);
        assert_eq!(stats.attempts_last_hour, 25);
        assert_eq!(stats.attempts_last_day, 100);
        assert_eq!(stats.unique_ips, 45);
        assert_eq!(stats.unique_users, 30);
    }

    #[test]
    fn test_rate_limit_stats_serialization() {
        let stats = RateLimitStats {
            total_attempts: 1000,
            successful_attempts: 950,
            failed_attempts: 50,
            attempts_last_hour: 120,
            attempts_last_day: 1000,
            unique_ips: 200,
            unique_users: 150,
        };

        let json = serde_json::to_string(&stats).expect("Failed to serialize RateLimitStats");
        assert!(json.contains("total_attempts"));
        assert!(json.contains("950"));
        assert!(json.contains("50"));
    }

    #[test]
    fn test_failing_ip_info_creation() {
        let ip_info = FailingIpInfo {
            ip_address: Some("192.168.1.100".to_string()),
            attempt_count: Some(25),
            failed_count: Some(20),
            last_attempt_at: Some(Utc::now()),
        };

        assert_eq!(ip_info.ip_address, Some("192.168.1.100".to_string()));
        assert_eq!(ip_info.attempt_count, Some(25));
        assert_eq!(ip_info.failed_count, Some(20));
        assert!(ip_info.last_attempt_at.is_some());
    }

    #[test]
    fn test_failing_ip_info_empty() {
        let ip_info = FailingIpInfo {
            ip_address: None,
            attempt_count: None,
            failed_count: None,
            last_attempt_at: None,
        };

        assert!(ip_info.ip_address.is_none());
        assert!(ip_info.attempt_count.is_none());
        assert!(ip_info.failed_count.is_none());
        assert!(ip_info.last_attempt_at.is_none());
    }

    #[test]
    fn test_failing_ip_info_serialization() {
        let ip_info = FailingIpInfo {
            ip_address: Some("10.0.0.5".to_string()),
            attempt_count: Some(50),
            failed_count: Some(35),
            last_attempt_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&ip_info).expect("Failed to serialize FailingIpInfo");
        assert!(json.contains("ip_address"));
        assert!(json.contains("10.0.0.5"));
        assert!(json.contains("attempt_count"));
        assert!(json.contains("50"));
    }

    #[test]
    fn test_rate_limit_query_deserialization() {
        let json = r#"{"limit": 10}"#;
        let query: RateLimitQuery = serde_json::from_str(json).expect("Failed to deserialize RateLimitQuery");
        assert_eq!(query.limit, Some(10));

        let json_no_limit = r#"{}"#;
        let query_no_limit: RateLimitQuery = serde_json::from_str(json_no_limit).expect("Failed to deserialize RateLimitQuery");
        assert!(query_no_limit.limit.is_none());
    }

    #[test]
    fn test_user_stats_creation() {
        let now = Utc::now();
        let stats = UserStats {
            failed_attempts_24h: 3,
            successful_attempts_24h: 15,
            last_attempt_at: Some(now),
        };

        assert_eq!(stats.failed_attempts_24h, 3);
        assert_eq!(stats.successful_attempts_24h, 15);
        assert_eq!(stats.last_attempt_at, Some(now));
    }

    #[test]
    fn test_user_stats_no_attempts() {
        let stats = UserStats {
            failed_attempts_24h: 0,
            successful_attempts_24h: 0,
            last_attempt_at: None,
        };

        assert_eq!(stats.failed_attempts_24h, 0);
        assert_eq!(stats.successful_attempts_24h, 0);
        assert!(stats.last_attempt_at.is_none());
    }

    #[test]
    fn test_user_stats_serialization() {
        let stats = UserStats {
            failed_attempts_24h: 5,
            successful_attempts_24h: 20,
            last_attempt_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&stats).expect("Failed to serialize UserStats");
        assert!(json.contains("failed_attempts_24h"));
        assert!(json.contains("successful_attempts_24h"));
        assert!(json.contains("last_attempt_at"));
    }

    #[test]
    fn test_auth_attempt_with_ipv6() {
        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("ipv6user".to_string()),
            success: true,
            user_agent: Some("Safari/14.0".to_string()),
            created_at: Utc::now(),
        };

        assert!(attempt.ip_address.unwrap().contains("2001:0db8"));
        assert_eq!(attempt.username.unwrap(), "ipv6user");
    }

    #[test]
    fn test_auth_attempt_with_long_user_agent() {
        let long_user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36 Edg/91.0.864.59".to_string();

        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("203.0.113.1".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("webuser".to_string()),
            success: true,
            user_agent: Some(long_user_agent.clone()),
            created_at: Utc::now(),
        };

        assert_eq!(attempt.user_agent.unwrap(), long_user_agent);
    }

    #[test]
    fn test_auth_attempt_with_unicode_username() {
        let attempt = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("192.168.1.50".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("用户名测试".to_string()),
            success: false,
            user_agent: Some("Mobile App v2.1".to_string()),
            created_at: Utc::now(),
        };

        assert_eq!(attempt.username.unwrap(), "用户名测试");
        assert!(!attempt.success);
    }

    #[test]
    fn test_rate_limit_result_with_zero_window() {
        let result = RateLimitResult {
            is_limited: false,
            remaining_attempts: 10,
            reset_time: None,
            window_minutes: 0,
        };

        assert_eq!(result.window_minutes, 0);
        assert!(!result.is_limited);
    }

    #[test]
    fn test_rate_limit_result_with_large_window() {
        let result = RateLimitResult {
            is_limited: true,
            remaining_attempts: 0,
            reset_time: Some(Utc::now() + Duration::hours(24)),
            window_minutes: 1440, // 24 hours
        };

        assert_eq!(result.window_minutes, 1440);
        assert!(result.is_limited);
    }

    #[test]
    fn test_rate_limit_stats_with_zero_values() {
        let stats = RateLimitStats {
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
            attempts_last_hour: 0,
            attempts_last_day: 0,
            unique_ips: 0,
            unique_users: 0,
        };

        assert_eq!(stats.total_attempts, 0);
        assert_eq!(stats.successful_attempts, 0);
        assert_eq!(stats.failed_attempts, 0);
    }

    #[test]
    fn test_rate_limit_stats_consistency() {
        let stats = RateLimitStats {
            total_attempts: 100,
            successful_attempts: 75,
            failed_attempts: 25,
            attempts_last_hour: 10,
            attempts_last_day: 50,
            unique_ips: 20,
            unique_users: 15,
        };

        // Total should equal successful + failed
        assert_eq!(stats.total_attempts, stats.successful_attempts + stats.failed_attempts);

        // Last hour should be <= last day <= total
        assert!(stats.attempts_last_hour <= stats.attempts_last_day);
        assert!(stats.attempts_last_day <= stats.total_attempts);
    }

    #[test]
    fn test_failing_ip_info_with_high_failure_rate() {
        let ip_info = FailingIpInfo {
            ip_address: Some("198.51.100.10".to_string()),
            attempt_count: Some(100),
            failed_count: Some(95),
            last_attempt_at: Some(Utc::now() - Duration::minutes(5)),
        };

        // High failure rate (95%)
        let failure_rate = ip_info.failed_count.unwrap() as f64 / ip_info.attempt_count.unwrap() as f64;
        assert!(failure_rate > 0.9);
        assert!(ip_info.last_attempt_at.unwrap() < Utc::now());
    }

    #[test]
    fn test_user_stats_calculation_helper() {
        let stats = UserStats {
            failed_attempts_24h: 8,
            successful_attempts_24h: 2,
            last_attempt_at: Some(Utc::now() - Duration::hours(2)),
        };

        // Calculate success rate
        let total_attempts = stats.failed_attempts_24h + stats.successful_attempts_24h;
        let success_rate = stats.successful_attempts_24h as f64 / total_attempts as f64;

        assert_eq!(total_attempts, 10);
        assert_eq!(success_rate, 0.2); // 20% success rate

        // Check if last attempt was recent (within 24 hours)
        let hours_since_last = Utc::now().signed_duration_since(stats.last_attempt_at.unwrap()).num_hours();
        assert!(hours_since_last < 24);
    }

    #[test]
    fn test_auth_attempt_time_ordering() {
        let now = Utc::now();
        let attempt1 = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("192.168.1.1".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("user1".to_string()),
            success: true,
            user_agent: None,
            created_at: now - Duration::minutes(10),
        };

        let attempt2 = AuthAttempt {
            id: Uuid::now_v7(),
            ip_address: Some("192.168.1.1".to_string()),
            user_id: Some(Uuid::now_v7()),
            username: Some("user2".to_string()),
            success: false,
            user_agent: None,
            created_at: now,
        };

        assert!(attempt1.created_at < attempt2.created_at);
        assert!(attempt2.created_at - attempt1.created_at == Duration::minutes(10));
    }
}