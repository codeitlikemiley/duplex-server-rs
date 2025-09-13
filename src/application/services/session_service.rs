use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    errors::AppError,
    infrastructure::auth::JwtService,
    PostgreSQL,
};

pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
    pub last_activity_at: chrono::DateTime<chrono::Utc>,
}

pub struct SessionService {
    db: PostgreSQL,
    jwt_service: JwtService,
}

impl SessionService {
    pub fn new(db: PostgreSQL) -> Self {
        Self {
            db,
            jwt_service: JwtService::default(),
        }
    }

    /// Hash a token for secure storage
    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Create a new session for a user
    pub async fn create_session(
        &self,
        user_id: Uuid,
        email: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<String, AppError> {
        // Generate JWT token
        let token = self
            .jwt_service
            .generate_token(user_id, email)
            .map_err(|e| AppError::Internal {
                message: format!("Failed to generate JWT: {}", e),
            })?;

        // Hash the token for storage
        let token_hash = Self::hash_token(&token);

        // Set session expiration (7 days by default)
        let expires_at = Utc::now() + Duration::days(7);

        // Store session in database
        sqlx::query!(
            r#"
            INSERT INTO user_sessions (id, user_id, token_hash, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::now_v7(),
            user_id,
            token_hash,
            ip_address,
            user_agent,
            expires_at
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create session: {}", e),
        })?;

        // Update user's last login time
        sqlx::query!(
            r#"
            UPDATE users
            SET last_login_at = $2
            WHERE id = $1
            "#,
            user_id,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update last login: {}", e),
        })?;

        Ok(token)
    }

    /// Validate a session token
    pub async fn validate_session(&self, token: &str) -> Result<Uuid, AppError> {
        // First validate the JWT structure and get user_id
        let claims = self
            .jwt_service
            .verify_token(token)
            .map_err(|_| AppError::Authentication {
                message: "Invalid or expired token".to_string(),
            })?;

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Authentication {
            message: "Invalid token format".to_string(),
        })?;

        // Hash the token to check against database
        let token_hash = Self::hash_token(token);

        // Check if session exists and is valid
        let session = sqlx::query!(
            r#"
            SELECT id, expires_at, revoked
            FROM user_sessions
            WHERE token_hash = $1 AND user_id = $2
            "#,
            token_hash,
            user_id
        )
        .fetch_optional(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch session: {}", e),
        })?;

        let session = session.ok_or_else(|| AppError::Authentication {
            message: "Session not found".to_string(),
        })?;

        // Check if session is revoked
        if session.revoked {
            return Err(AppError::Authentication {
                message: "Session has been revoked".to_string(),
            });
        }

        // Check if session is expired
        if session.expires_at < Utc::now() {
            return Err(AppError::Authentication {
                message: "Session has expired".to_string(),
            });
        }

        // Update last activity time
        sqlx::query!(
            r#"
            UPDATE user_sessions
            SET last_activity_at = $2
            WHERE id = $1
            "#,
            session.id,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to update session activity: {}", e),
        })?;

        Ok(user_id)
    }

    /// Revoke a specific session
    pub async fn revoke_session(&self, token: &str) -> Result<(), AppError> {
        let token_hash = Self::hash_token(token);

        sqlx::query!(
            r#"
            UPDATE user_sessions
            SET revoked = true
            WHERE token_hash = $1
            "#,
            token_hash
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to revoke session: {}", e),
        })?;

        Ok(())
    }

    /// Revoke all sessions for a user (logout from all devices)
    pub async fn revoke_all_user_sessions(&self, user_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"
            UPDATE user_sessions
            SET revoked = true
            WHERE user_id = $1 AND revoked = false
            "#,
            user_id
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to revoke user sessions: {}", e),
        })?;

        Ok(result.rows_affected())
    }

    /// Get all active sessions for a user
    pub async fn get_user_sessions(&self, user_id: Uuid) -> Result<Vec<UserSession>, AppError> {
        let sessions = sqlx::query_as!(
            UserSession,
            r#"
            SELECT id, user_id, token_hash, ip_address, user_agent,
                   expires_at, revoked, last_activity_at
            FROM user_sessions
            WHERE user_id = $1 AND revoked = false AND expires_at > $2
            ORDER BY last_activity_at DESC
            "#,
            user_id,
            Utc::now()
        )
        .fetch_all(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch user sessions: {}", e),
        })?;

        Ok(sessions)
    }

    /// Clean up expired sessions (can be run periodically)
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_sessions
            WHERE expires_at < $1 OR revoked = true
            "#,
            Utc::now()
        )
        .execute(&self.db.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to cleanup expired sessions: {}", e),
        })?;

        Ok(result.rows_affected())
    }
}