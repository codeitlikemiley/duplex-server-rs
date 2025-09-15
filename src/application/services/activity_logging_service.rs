use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::{
    errors::AppError,
};

pub struct ActivityLoggingService {
    db: Pool<Postgres>,
}

impl ActivityLoggingService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { db: pool }
    }

    /// Log a user activity
    pub async fn log_activity(&self, activity: UserActivity) -> Result<(), AppError> {
        let metadata_json = serde_json::to_value(&activity.metadata)
            .map_err(|e| AppError::Internal {
                message: format!("Failed to serialize activity metadata: {}", e)
            })?;

        // IP address is stored as string
        let ip_addr = activity.ip_address.as_deref();

        sqlx::query!(
            r#"
            INSERT INTO user_activities (id, user_id, activity_type, description, ip_address, user_agent, metadata, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            activity.id,
            activity.user_id,
            activity.activity_type as ActivityType,
            activity.description,
            ip_addr,
            activity.user_agent,
            metadata_json,
            activity.created_at
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to log user activity: {}", e)
        })?;

        Ok(())
    }

    /// Get user activities with pagination
    pub async fn get_user_activities(
        &self,
        user_id: Uuid,
        filters: ActivityFilters,
    ) -> Result<ActivityResult, AppError> {
        let mut query = String::from(
            "SELECT id, user_id, activity_type as \"activity_type: ActivityType\", description,
                    ip_address, user_agent, metadata, created_at
             FROM user_activities
             WHERE user_id = $1"
        );

        let mut conditions = Vec::new();
        let mut param_count = 1;

        if let Some(ref activity_type) = filters.activity_type {
            param_count += 1;
            conditions.push(format!("AND activity_type = ${}", param_count));
        }

        if let Some(ref from_date) = filters.from_date {
            param_count += 1;
            conditions.push(format!("AND created_at >= ${}", param_count));
        }

        if let Some(ref to_date) = filters.to_date {
            param_count += 1;
            conditions.push(format!("AND created_at <= ${}", param_count));
        }

        if let Some(ref ip_address) = filters.ip_address {
            param_count += 1;
            conditions.push(format!("AND ip_address = ${}", param_count));
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" {}", conditions.join(" ")));
        }

        query.push_str(" ORDER BY created_at DESC");

        let limit = filters.limit.unwrap_or(50).min(100);
        let offset = filters.offset.unwrap_or(0);
        param_count += 1;
        query.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));

        let mut sql_query = sqlx::query_as::<_, UserActivityRecord>(&query);
        sql_query = sql_query.bind(user_id);

        if let Some(ref activity_type) = filters.activity_type {
            sql_query = sql_query.bind(activity_type);
        }

        if let Some(ref from_date) = filters.from_date {
            sql_query = sql_query.bind(from_date);
        }

        if let Some(ref to_date) = filters.to_date {
            sql_query = sql_query.bind(to_date);
        }

        if let Some(ref ip_address) = filters.ip_address {
            sql_query = sql_query.bind(ip_address);
        }

        sql_query = sql_query.bind(limit);
        sql_query = sql_query.bind(offset);

        let activities = sql_query
            .fetch_all(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to fetch user activities: {}", e)
            })?;

        let total_count = self.count_user_activities(user_id, &filters).await?;

        Ok(ActivityResult {
            activities: activities.into_iter().map(|a| a.into()).collect(),
            total_count,
            limit,
            offset,
            has_more: (offset + limit as i64) < total_count,
        })
    }

    /// Count user activities matching filters
    async fn count_user_activities(
        &self,
        user_id: Uuid,
        filters: &ActivityFilters,
    ) -> Result<i64, AppError> {
        let mut query = String::from(
            "SELECT COUNT(*) FROM user_activities WHERE user_id = $1"
        );

        let mut conditions = Vec::new();
        let mut param_count = 1;

        if let Some(ref activity_type) = filters.activity_type {
            param_count += 1;
            conditions.push(format!("AND activity_type = ${}", param_count));
        }

        if let Some(ref from_date) = filters.from_date {
            param_count += 1;
            conditions.push(format!("AND created_at >= ${}", param_count));
        }

        if let Some(ref to_date) = filters.to_date {
            param_count += 1;
            conditions.push(format!("AND created_at <= ${}", param_count));
        }

        if let Some(ref ip_address) = filters.ip_address {
            param_count += 1;
            conditions.push(format!("AND ip_address = ${}", param_count));
        }

        if !conditions.is_empty() {
            query.push_str(&format!(" {}", conditions.join(" ")));
        }

        let mut sql_query = sqlx::query_scalar::<_, i64>(&query);
        sql_query = sql_query.bind(user_id);

        if let Some(ref activity_type) = filters.activity_type {
            sql_query = sql_query.bind(activity_type);
        }

        if let Some(ref from_date) = filters.from_date {
            sql_query = sql_query.bind(from_date);
        }

        if let Some(ref to_date) = filters.to_date {
            sql_query = sql_query.bind(to_date);
        }

        if let Some(ref ip_address) = filters.ip_address {
            sql_query = sql_query.bind(ip_address);
        }

        let count = sql_query
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to count user activities: {}", e)
            })?;

        Ok(count)
    }

    /// Get activity statistics for a user
    pub async fn get_user_activity_stats(&self, user_id: Uuid) -> Result<ActivityStats, AppError> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_activities,
                COUNT(CASE WHEN activity_type = 'Login' THEN 1 END) as login_count,
                COUNT(CASE WHEN activity_type = 'Logout' THEN 1 END) as logout_count,
                COUNT(CASE WHEN activity_type = 'ProfileUpdate' THEN 1 END) as profile_updates,
                COUNT(CASE WHEN activity_type = 'PasswordChange' THEN 1 END) as password_changes,
                COUNT(CASE WHEN activity_type = 'AccountDeactivation' THEN 1 END) as account_deactivations,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '7 days' THEN 1 END) as activities_last_7_days,
                COUNT(CASE WHEN created_at >= NOW() - INTERVAL '30 days' THEN 1 END) as activities_last_30_days,
                MAX(created_at) as last_activity_at,
                COUNT(DISTINCT ip_address) as unique_ip_addresses
            FROM user_activities
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to get activity statistics: {}", e)
        })?;

        Ok(ActivityStats {
            total_activities: stats.total_activities.unwrap_or(0),
            login_count: stats.login_count.unwrap_or(0),
            logout_count: stats.logout_count.unwrap_or(0),
            profile_updates: stats.profile_updates.unwrap_or(0),
            password_changes: stats.password_changes.unwrap_or(0),
            account_deactivations: stats.account_deactivations.unwrap_or(0),
            activities_last_7_days: stats.activities_last_7_days.unwrap_or(0),
            activities_last_30_days: stats.activities_last_30_days.unwrap_or(0),
            last_activity_at: stats.last_activity_at,
            unique_ip_addresses: stats.unique_ip_addresses.unwrap_or(0),
        })
    }

    /// Delete old activities (cleanup job)
    pub async fn cleanup_old_activities(&self, older_than_days: i32) -> Result<u64, AppError> {
        let deleted_count = sqlx::query!(
            "DELETE FROM user_activities WHERE created_at < NOW() - INTERVAL '1 day' * $1",
            older_than_days as f64
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup old activities: {}", e)
        })?
        .rows_affected();

        Ok(deleted_count)
    }
}

// Helper functions for creating common activities
impl ActivityLoggingService {
    pub async fn log_login(
        &self,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
        success: bool,
    ) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert("success".to_string(), serde_json::Value::Bool(success));

        let activity = UserActivity {
            id: Uuid::now_v7(),
            user_id,
            activity_type: ActivityType::Login,
            description: if success {
                "User logged in successfully".to_string()
            } else {
                "User login attempt failed".to_string()
            },
            ip_address,
            user_agent,
            metadata,
            created_at: Utc::now(),
        };

        self.log_activity(activity).await
    }

    pub async fn log_logout(
        &self,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), AppError> {
        let activity = UserActivity {
            id: Uuid::now_v7(),
            user_id,
            activity_type: ActivityType::Logout,
            description: "User logged out".to_string(),
            ip_address,
            user_agent,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };

        self.log_activity(activity).await
    }

    pub async fn log_password_change(
        &self,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), AppError> {
        let activity = UserActivity {
            id: Uuid::now_v7(),
            user_id,
            activity_type: ActivityType::PasswordChange,
            description: "User changed password".to_string(),
            ip_address,
            user_agent,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };

        self.log_activity(activity).await
    }

    pub async fn log_profile_update(
        &self,
        user_id: Uuid,
        updated_fields: Vec<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), AppError> {
        let mut metadata = HashMap::new();
        metadata.insert(
            "updated_fields".to_string(),
            serde_json::Value::Array(
                updated_fields.into_iter().map(serde_json::Value::String).collect()
            )
        );

        let activity = UserActivity {
            id: Uuid::now_v7(),
            user_id,
            activity_type: ActivityType::ProfileUpdate,
            description: "User updated profile".to_string(),
            ip_address,
            user_agent,
            metadata,
            created_at: Utc::now(),
        };

        self.log_activity(activity).await
    }

    pub async fn log_account_deactivation(
        &self,
        user_id: Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), AppError> {
        let activity = UserActivity {
            id: Uuid::now_v7(),
            user_id,
            activity_type: ActivityType::AccountDeactivation,
            description: "User deactivated account".to_string(),
            ip_address,
            user_agent,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };

        self.log_activity(activity).await
    }
}

// DTOs and Types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "activity_type", rename_all = "PascalCase")]
pub enum ActivityType {
    Login,
    Logout,
    PasswordChange,
    ProfileUpdate,
    AccountDeactivation,
    AccountReactivation,
    AccountDeletion,
    EmailVerification,
    PasswordReset,
    SessionExpired,
    SecurityAlert,
}

#[derive(Debug, Clone)]
pub struct UserActivity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub activity_type: ActivityType,
    pub description: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserActivityRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub activity_type: ActivityType,
    pub description: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<UserActivityRecord> for UserActivityResponse {
    fn from(record: UserActivityRecord) -> Self {
        Self {
            id: record.id,
            activity_type: record.activity_type,
            description: record.description,
            ip_address: record.ip_address,
            user_agent: record.user_agent,
            metadata: record.metadata,
            created_at: record.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserActivityResponse {
    pub id: Uuid,
    pub activity_type: ActivityType,
    pub description: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityFilters {
    pub activity_type: Option<ActivityType>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ActivityResult {
    pub activities: Vec<UserActivityResponse>,
    pub total_count: i64,
    pub limit: i32,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct ActivityStats {
    pub total_activities: i64,
    pub login_count: i64,
    pub logout_count: i64,
    pub profile_updates: i64,
    pub password_changes: i64,
    pub account_deactivations: i64,
    pub activities_last_7_days: i64,
    pub activities_last_30_days: i64,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub unique_ip_addresses: i64,
}