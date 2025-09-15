use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

use crate::errors::AppError;

pub struct RbacService {
    db: Pool<Postgres>,
}

impl RbacService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { db: pool }
    }

    /// Check if a user has a specific permission
    pub async fn has_permission(
        &self,
        user_id: Uuid,
        resource: &str,
        action: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query_scalar!(
            "SELECT user_has_permission($1, $2, $3)",
            user_id,
            resource,
            action
        )
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to check user permission: {}", e),
        })?;

        Ok(result.unwrap_or(false))
    }

    /// Get all permissions for a user
    pub async fn get_user_permissions(&self, user_id: Uuid) -> Result<Vec<Permission>, AppError> {
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT DISTINCT p.id, p.resource, p.action, p.description, p.created_at
            FROM user_roles ur
            JOIN role_permissions rp ON ur.role_id = rp.role_id
            JOIN permissions p ON rp.permission_id = p.id
            WHERE ur.user_id = $1
                AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
            ORDER BY p.resource, p.action
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch user permissions: {}", e),
        })?;

        Ok(permissions)
    }

    /// Get all roles for a user
    pub async fn get_user_roles(&self, user_id: Uuid) -> Result<Vec<UserRole>, AppError> {
        let roles = sqlx::query_as!(
            UserRole,
            r#"
            SELECT r.id, r.name, r.description, r.is_system,
                   ur.assigned_at, ur.expires_at
            FROM user_roles ur
            JOIN roles r ON ur.role_id = r.id
            WHERE ur.user_id = $1
                AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
            ORDER BY r.name
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch user roles: {}", e),
        })?;

        Ok(roles)
    }

    /// Assign a role to a user
    pub async fn assign_role(
        &self,
        user_id: Uuid,
        role_name: &str,
        assigned_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        // First get the role ID
        let role_id = sqlx::query_scalar!(
            "SELECT id FROM roles WHERE name = $1",
            role_name
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find role: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Role".to_string(),
            id: Some(role_name.to_string()),
        })?;

        // Assign the role
        sqlx::query!(
            r#"
            INSERT INTO user_roles (user_id, role_id, assigned_at, assigned_by, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, role_id)
            DO UPDATE SET
                assigned_at = EXCLUDED.assigned_at,
                assigned_by = EXCLUDED.assigned_by,
                expires_at = EXCLUDED.expires_at
            "#,
            user_id,
            role_id,
            Utc::now(),
            assigned_by,
            expires_at
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to assign role: {}", e),
        })?;

        Ok(())
    }

    /// Remove a role from a user
    pub async fn revoke_role(&self, user_id: Uuid, role_name: &str) -> Result<(), AppError> {
        // First get the role ID
        let role_id = sqlx::query_scalar!(
            "SELECT id FROM roles WHERE name = $1",
            role_name
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find role: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Role".to_string(),
            id: Some(role_name.to_string()),
        })?;

        // Remove the role
        let result = sqlx::query!(
            "DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2",
            user_id,
            role_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to revoke role: {}", e),
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound {
                resource: "User role assignment".to_string(),
                id: Some(format!("{}-{}", user_id, role_name)),
            });
        }

        Ok(())
    }

    /// Create a new custom role
    pub async fn create_role(
        &self,
        name: &str,
        description: Option<String>,
        permissions: Vec<PermissionGrant>,
    ) -> Result<Role, AppError> {
        let role_id = Uuid::now_v7();

        // Start a transaction
        let mut tx = self.db.begin().await.map_err(|e| AppError::Database {
            message: format!("Failed to start transaction: {}", e),
        })?;

        // Create the role
        sqlx::query!(
            r#"
            INSERT INTO roles (id, name, description, is_system, created_at, updated_at)
            VALUES ($1, $2, $3, false, $4, $4)
            "#,
            role_id,
            name,
            description,
            Utc::now()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create role: {}", e),
        })?;

        // Assign permissions to the role
        for perm in permissions {
            let permission_id = sqlx::query_scalar!(
                "SELECT id FROM permissions WHERE resource = $1 AND action = $2",
                perm.resource,
                perm.action
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to find permission: {}", e),
            })?
            .ok_or_else(|| AppError::NotFound {
                resource: "Permission".to_string(),
                id: Some(format!("{}:{}", perm.resource, perm.action)),
            })?;

            sqlx::query!(
                r#"
                INSERT INTO role_permissions (role_id, permission_id, granted_at)
                VALUES ($1, $2, $3)
                "#,
                role_id,
                permission_id,
                Utc::now()
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database {
                message: format!("Failed to assign permission to role: {}", e),
            })?;
        }

        // Commit the transaction
        tx.commit().await.map_err(|e| AppError::Database {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(Role {
            id: role_id,
            name: name.to_string(),
            description,
            is_system: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Delete a custom role
    pub async fn delete_role(&self, role_name: &str) -> Result<(), AppError> {
        // Check if role is a system role
        let is_system = sqlx::query_scalar!(
            "SELECT is_system FROM roles WHERE name = $1",
            role_name
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find role: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Role".to_string(),
            id: Some(role_name.to_string()),
        })?;

        if is_system {
            return Err(AppError::Validation {
                field: "role".to_string(),
                message: "Cannot delete system roles".to_string(),
            });
        }

        // Delete the role (cascade will handle related records)
        sqlx::query!(
            "DELETE FROM roles WHERE name = $1 AND is_system = false",
            role_name
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to delete role: {}", e),
        })?;

        Ok(())
    }

    /// Get all available roles
    pub async fn get_all_roles(&self) -> Result<Vec<Role>, AppError> {
        let roles = sqlx::query_as!(
            Role,
            r#"
            SELECT id, name, description, is_system, created_at, updated_at
            FROM roles
            ORDER BY is_system DESC, name
            "#
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch roles: {}", e),
        })?;

        Ok(roles)
    }

    /// Get all available permissions
    pub async fn get_all_permissions(&self) -> Result<Vec<Permission>, AppError> {
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT id, resource, action, description, created_at
            FROM permissions
            ORDER BY resource, action
            "#
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch permissions: {}", e),
        })?;

        Ok(permissions)
    }

    /// Get permissions for a specific role
    pub async fn get_role_permissions(&self, role_name: &str) -> Result<Vec<Permission>, AppError> {
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT p.id, p.resource, p.action, p.description, p.created_at
            FROM permissions p
            JOIN role_permissions rp ON p.id = rp.permission_id
            JOIN roles r ON rp.role_id = r.id
            WHERE r.name = $1
            ORDER BY p.resource, p.action
            "#,
            role_name
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch role permissions: {}", e),
        })?;

        Ok(permissions)
    }

    /// Add permission to an existing role
    pub async fn grant_permission_to_role(
        &self,
        role_name: &str,
        resource: &str,
        action: &str,
        granted_by: Uuid,
    ) -> Result<(), AppError> {
        // Get role ID
        let role_id = sqlx::query_scalar!(
            "SELECT id FROM roles WHERE name = $1",
            role_name
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find role: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Role".to_string(),
            id: Some(role_name.to_string()),
        })?;

        // Get permission ID
        let permission_id = sqlx::query_scalar!(
            "SELECT id FROM permissions WHERE resource = $1 AND action = $2",
            resource,
            action
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find permission: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Permission".to_string(),
            id: Some(format!("{}:{}", resource, action)),
        })?;

        // Grant the permission
        sqlx::query!(
            r#"
            INSERT INTO role_permissions (role_id, permission_id, granted_at, granted_by)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
            "#,
            role_id,
            permission_id,
            Utc::now(),
            granted_by
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to grant permission: {}", e),
        })?;

        Ok(())
    }

    /// Remove permission from a role
    pub async fn revoke_permission_from_role(
        &self,
        role_name: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), AppError> {
        // Get role ID
        let role_id = sqlx::query_scalar!(
            "SELECT id FROM roles WHERE name = $1",
            role_name
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find role: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Role".to_string(),
            id: Some(role_name.to_string()),
        })?;

        // Get permission ID
        let permission_id = sqlx::query_scalar!(
            "SELECT id FROM permissions WHERE resource = $1 AND action = $2",
            resource,
            action
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to find permission: {}", e),
        })?
        .ok_or_else(|| AppError::NotFound {
            resource: "Permission".to_string(),
            id: Some(format!("{}:{}", resource, action)),
        })?;

        // Revoke the permission
        sqlx::query!(
            "DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2",
            role_id,
            permission_id
        )
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to revoke permission: {}", e),
        })?;

        Ok(())
    }

    /// Bulk check multiple permissions
    pub async fn has_any_permission(
        &self,
        user_id: Uuid,
        required_permissions: Vec<(String, String)>,
    ) -> Result<bool, AppError> {
        for (resource, action) in required_permissions {
            if self.has_permission(user_id, &resource, &action).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if user has all specified permissions
    pub async fn has_all_permissions(
        &self,
        user_id: Uuid,
        required_permissions: Vec<(String, String)>,
    ) -> Result<bool, AppError> {
        for (resource, action) in required_permissions {
            if !self.has_permission(user_id, &resource, &action).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get users with a specific role
    pub async fn get_users_by_role(&self, role_name: &str) -> Result<Vec<UserWithRole>, AppError> {
        let users = sqlx::query_as!(
            UserWithRole,
            r#"
            SELECT u.id, u.username, u.email, ur.assigned_at, ur.expires_at
            FROM users u
            JOIN user_roles ur ON u.id = ur.user_id
            JOIN roles r ON ur.role_id = r.id
            WHERE r.name = $1
                AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
            ORDER BY ur.assigned_at DESC
            "#,
            role_name
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to fetch users by role: {}", e),
        })?;

        Ok(users)
    }
}

// DTOs
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRole {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserWithRole {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: Uuid,
    pub role_name: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<PermissionGrant>,
}