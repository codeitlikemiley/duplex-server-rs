use chrono::{DateTime, Utc, Duration};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::{
    errors::AppError,
    models::UserStatus,
};

pub struct AccountLockoutService {
    db: Pool<Postgres>,
}

impl AccountLockoutService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { db: pool }
    }

    /// Check if an account is locked out
    pub async fn is_account_locked(&self, user_id: Uuid) -> Result<LockoutStatus, AppError> {
        let lockout_info = sqlx::query_as!(
            AccountLockoutRecord,
            r#"
            SELECT id, user_id, lockout_type as "lockout_type: LockoutType",
                   reason, locked_at, locked_until, unlock_attempts, is_active,
                   created_by, created_at
            FROM account_lockouts
            WHERE user_id = $1 AND is_active = true
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to check account lockout: {}", e)
        })?;

        match lockout_info {
            Some(lockout) => {
                // Check if temporary lockout has expired
                if lockout.lockout_type == LockoutType::Temporary {
                    if let Some(locked_until) = lockout.locked_until {
                        if Utc::now() > locked_until {
                            // Lockout has expired, deactivate it
                            self.deactivate_lockout(lockout.id).await?;
                            return Ok(LockoutStatus {
                                is_locked: false,
                                lockout_type: None,
                                reason: None,
                                locked_until: None,
                                unlock_attempts: 0,
                            });
                        }
                    }
                }

                Ok(LockoutStatus {
                    is_locked: true,
                    lockout_type: Some(lockout.lockout_type),
                    reason: Some(lockout.reason),
                    locked_until: lockout.locked_until,
                    unlock_attempts: lockout.unlock_attempts,
                })
            }
            None => Ok(LockoutStatus {
                is_locked: false,
                lockout_type: None,
                reason: None,
                locked_until: None,
                unlock_attempts: 0,
            }),
        }
    }

    /// Lock an account due to failed login attempts
    pub async fn lock_account_for_failed_attempts(&self, user_id: Uuid, failed_attempts: i32) -> Result<LockoutInfo, AppError> {
        // Determine lockout type and duration based on failed attempts
        let (lockout_type, duration_minutes, reason) = match failed_attempts {
            5..=9 => (LockoutType::Temporary, 30, "Account temporarily locked due to multiple failed login attempts"),
            10..=14 => (LockoutType::Temporary, 120, "Account temporarily locked due to excessive failed login attempts"),
            15..=19 => (LockoutType::Temporary, 1440, "Account temporarily locked for 24 hours due to repeated failed attempts"),
            _ => (LockoutType::Permanent, 0, "Account permanently locked due to suspicious activity"),
        };

        let locked_until = if lockout_type == LockoutType::Temporary {
            Some(Utc::now() + Duration::minutes(duration_minutes))
        } else {
            None
        };

        let lockout_id = Uuid::now_v7();

        // Insert lockout record
        sqlx::query!(
            r#"
            INSERT INTO account_lockouts (id, user_id, lockout_type, reason, locked_at, locked_until,
                                        unlock_attempts, is_active, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, true, $7, $8)
            "#,
            lockout_id,
            user_id,
            lockout_type.clone() as LockoutType,
            reason,
            Utc::now(),
            locked_until,
            user_id, // Self-initiated lockout
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create account lockout: {}", e)
        })?;

        // Update user status to suspended if permanent lockout
        if lockout_type == LockoutType::Permanent {
            sqlx::query!(
                "UPDATE users SET status = $1 WHERE id = $2",
                UserStatus::Suspended as UserStatus,
                user_id
            )
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to update user status: {}", e)
            })?;
        }

        Ok(LockoutInfo {
            lockout_id,
            lockout_type: lockout_type.clone(),
            reason: reason.to_string(),
            locked_until,
            duration_minutes: if lockout_type == LockoutType::Temporary { Some(duration_minutes as i32) } else { None },
        })
    }

    /// Manually lock an account (admin function)
    pub async fn manually_lock_account(
        &self,
        user_id: Uuid,
        admin_id: Uuid,
        lockout_type: LockoutType,
        reason: String,
        duration_hours: Option<i32>
    ) -> Result<LockoutInfo, AppError> {
        let locked_until = if lockout_type == LockoutType::Temporary {
            if let Some(hours) = duration_hours {
                Some(Utc::now() + Duration::hours(hours as i64))
            } else {
                Some(Utc::now() + Duration::hours(24)) // Default 24 hours
            }
        } else {
            None
        };

        let lockout_id = Uuid::now_v7();

        // Insert lockout record
        sqlx::query!(
            r#"
            INSERT INTO account_lockouts (id, user_id, lockout_type, reason, locked_at, locked_until,
                                        unlock_attempts, is_active, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, true, $7, $8)
            "#,
            lockout_id,
            user_id,
            lockout_type.clone() as LockoutType,
            reason,
            Utc::now(),
            locked_until,
            admin_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create manual account lockout: {}", e)
        })?;

        // Update user status if permanent lockout
        if lockout_type == LockoutType::Permanent {
            sqlx::query!(
                "UPDATE users SET status = $1 WHERE id = $2",
                UserStatus::Suspended as UserStatus,
                user_id
            )
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to update user status: {}", e)
            })?;
        }

        Ok(LockoutInfo {
            lockout_id,
            lockout_type,
            reason,
            locked_until,
            duration_minutes: duration_hours.map(|h| h * 60),
        })
    }

    /// Unlock an account (admin function)
    pub async fn unlock_account(&self, user_id: Uuid, admin_id: Uuid, reason: String) -> Result<(), AppError> {
        // Deactivate all active lockouts for this user
        sqlx::query!(
            "UPDATE account_lockouts SET is_active = false WHERE user_id = $1 AND is_active = true",
            user_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to deactivate lockouts: {}", e)
        })?;

        // Create unlock record
        sqlx::query!(
            r#"
            INSERT INTO account_unlocks (id, user_id, reason, unlocked_by, unlocked_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::now_v7(),
            user_id,
            reason,
            admin_id,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create unlock record: {}", e)
        })?;

        // Reactivate user if they were suspended
        sqlx::query!(
            "UPDATE users SET status = $1 WHERE id = $2 AND status = $3",
            UserStatus::Active as UserStatus,
            user_id,
            UserStatus::Suspended as UserStatus
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to reactivate user: {}", e)
        })?;

        Ok(())
    }

    /// Record an unlock attempt (user trying to access locked account)
    pub async fn record_unlock_attempt(&self, user_id: Uuid, ip_address: Option<String>) -> Result<(), AppError> {
        // Increment unlock attempts for active lockouts
        sqlx::query!(
            "UPDATE account_lockouts SET unlock_attempts = unlock_attempts + 1 WHERE user_id = $1 AND is_active = true",
            user_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to record unlock attempt: {}", e)
        })?;

        // Log the attempt
        sqlx::query!(
            r#"
            INSERT INTO unlock_attempts (id, user_id, ip_address, attempted_at)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::now_v7(),
            user_id,
            ip_address,
            Utc::now()
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to log unlock attempt: {}", e)
        })?;

        Ok(())
    }

    /// Get lockout history for a user
    pub async fn get_lockout_history(&self, user_id: Uuid, limit: i32) -> Result<Vec<LockoutHistoryItem>, AppError> {
        let history = sqlx::query_as!(
            LockoutHistoryItem,
            r#"
            SELECT
                al.id,
                al.lockout_type as "lockout_type: LockoutType",
                al.reason,
                al.locked_at,
                al.locked_until,
                al.unlock_attempts,
                al.is_active,
                au.unlocked_at,
                au.reason as unlock_reason,
                COALESCE(creator.username, 'System') as created_by_username,
                COALESCE(unlocker.username, 'System') as unlocked_by_username
            FROM account_lockouts al
            LEFT JOIN account_unlocks au ON al.user_id = au.user_id AND au.unlocked_at > al.locked_at
            LEFT JOIN users creator ON al.created_by = creator.id
            LEFT JOIN users unlocker ON au.unlocked_by = unlocker.id
            WHERE al.user_id = $1
            ORDER BY al.created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit as i64
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get lockout history: {}", e)
        })?;

        Ok(history)
    }

    /// Get lockout statistics (admin function)
    pub async fn get_lockout_stats(&self) -> Result<LockoutStats, AppError> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_lockouts,
                COUNT(CASE WHEN lockout_type = 'Temporary' THEN 1 END) as temporary_lockouts,
                COUNT(CASE WHEN lockout_type = 'Permanent' THEN 1 END) as permanent_lockouts,
                COUNT(CASE WHEN is_active = true THEN 1 END) as active_lockouts,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '24 hours' THEN 1 END) as lockouts_last_24h,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '7 days' THEN 1 END) as lockouts_last_7d,
                COUNT(DISTINCT user_id) as unique_users_locked
            FROM account_lockouts
            WHERE created_at >= NOW() - INTERVAL '30 days'
            "#
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get lockout statistics: {}", e)
        })?;

        Ok(LockoutStats {
            total_lockouts: stats.total_lockouts.unwrap_or(0),
            temporary_lockouts: stats.temporary_lockouts.unwrap_or(0),
            permanent_lockouts: stats.permanent_lockouts.unwrap_or(0),
            active_lockouts: stats.active_lockouts.unwrap_or(0),
            lockouts_last_24h: stats.lockouts_last_24h.unwrap_or(0),
            lockouts_last_7d: stats.lockouts_last_7d.unwrap_or(0),
            unique_users_locked: stats.unique_users_locked.unwrap_or(0),
        })
    }

    /// Cleanup expired temporary lockouts
    pub async fn cleanup_expired_lockouts(&self) -> Result<u64, AppError> {
        let updated = sqlx::query!(
            r#"
            UPDATE account_lockouts
            SET is_active = false
            WHERE lockout_type = 'Temporary'
              AND locked_until IS NOT NULL
              AND locked_until < NOW()
              AND is_active = true
            "#
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup expired lockouts: {}", e)
        })?
        .rows_affected();

        Ok(updated)
    }

    // Private helper method
    async fn deactivate_lockout(&self, lockout_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE account_lockouts SET is_active = false WHERE id = $1",
            lockout_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to deactivate lockout: {}", e)
        })?;

        Ok(())
    }
}

// DTOs and Types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "lockout_type", rename_all = "PascalCase")]
pub enum LockoutType {
    Temporary,
    Permanent,
}

#[derive(Debug, Serialize)]
pub struct LockoutStatus {
    pub is_locked: bool,
    pub lockout_type: Option<LockoutType>,
    pub reason: Option<String>,
    pub locked_until: Option<DateTime<Utc>>,
    pub unlock_attempts: i32,
}

#[derive(Debug, Serialize)]
pub struct LockoutInfo {
    pub lockout_id: Uuid,
    pub lockout_type: LockoutType,
    pub reason: String,
    pub locked_until: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AccountLockoutRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lockout_type: LockoutType,
    pub reason: String,
    pub locked_at: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub unlock_attempts: i32,
    pub is_active: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LockoutHistoryItem {
    pub id: Uuid,
    pub lockout_type: LockoutType,
    pub reason: String,
    pub locked_at: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub unlock_attempts: i32,
    pub is_active: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub unlock_reason: Option<String>,
    pub created_by_username: Option<String>,
    pub unlocked_by_username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LockoutStats {
    pub total_lockouts: i64,
    pub temporary_lockouts: i64,
    pub permanent_lockouts: i64,
    pub active_lockouts: i64,
    pub lockouts_last_24h: i64,
    pub lockouts_last_7d: i64,
    pub unique_users_locked: i64,
}

#[derive(Debug, Deserialize)]
pub struct ManualLockoutRequest {
    pub lockout_type: LockoutType,
    pub reason: String,
    pub duration_hours: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UnlockAccountRequest {
    pub reason: String,
}