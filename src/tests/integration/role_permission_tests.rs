//! Integration tests for role and permission assignments

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub failed_login_attempts: u32,
    pub account_locked_until: Option<DateTime<Utc>>,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    pub recovery_email: Option<String>,
    pub recovery_email_verified: bool,
    pub phone_number: Option<String>,
    pub phone_verified: bool,
    pub profile_picture_url: Option<String>,
    pub bio: Option<String>,
    pub preferences: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum AppError {
    NotFound {
        resource: String,
        id: Option<String>,
    },
    Unauthorized {
        message: String,
    },
    BadRequest {
        message: String,
    },
    InternalServerError {
        message: String,
    },
    Conflict {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub system_role: bool,
    pub permissions: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub resource: String,
    pub action: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub assigned_by: Uuid,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub context: Option<String>, // For context-specific roles
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub resource_id: Option<String>,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conditions: Option<serde_json::Value>,
}

pub struct RolePermissionSimulator {
    users: Vec<User>,
    roles: HashMap<Uuid, Role>,
    permissions: HashMap<Uuid, Permission>,
    role_assignments: HashMap<Uuid, Vec<RoleAssignment>>,
    permission_grants: HashMap<Uuid, Vec<PermissionGrant>>,
    permission_cache: HashMap<Uuid, HashSet<String>>,
    role_hierarchies: HashMap<Uuid, Vec<Uuid>>, // role_id -> parent_roles
}

impl RolePermissionSimulator {
    pub fn new() -> Self {
        let mut simulator = Self {
            users: vec![],
            roles: HashMap::new(),
            permissions: HashMap::new(),
            role_assignments: HashMap::new(),
            permission_grants: HashMap::new(),
            permission_cache: HashMap::new(),
            role_hierarchies: HashMap::new(),
        };

        // Create default system roles and permissions
        simulator.create_default_roles_and_permissions();
        simulator
    }

    fn create_default_roles_and_permissions(&mut self) {
        // Create default permissions
        let permissions = vec![
            ("user.read", "Read user information", "user", "read"),
            ("user.write", "Create and update users", "user", "write"),
            ("user.delete", "Delete users", "user", "delete"),
            ("role.read", "Read roles", "role", "read"),
            ("role.write", "Create and update roles", "role", "write"),
            ("role.assign", "Assign roles to users", "role", "assign"),
            ("admin.full", "Full administrative access", "system", "admin"),
            ("content.read", "Read content", "content", "read"),
            ("content.write", "Create and edit content", "content", "write"),
            ("content.publish", "Publish content", "content", "publish"),
        ];

        for (name, description, resource, action) in permissions {
            let permission = Permission {
                id: Uuid::now_v7(),
                name: name.to_string(),
                description: description.to_string(),
                resource: resource.to_string(),
                action: action.to_string(),
                created_at: Utc::now(),
            };
            self.permissions.insert(permission.id, permission);
        }

        // Create default roles
        let admin_permissions: HashSet<String> = self.permissions.values()
            .map(|p| p.name.clone())
            .collect();

        let admin_role = Role {
            id: Uuid::now_v7(),
            name: "admin".to_string(),
            description: "System administrator".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            system_role: true,
            permissions: admin_permissions,
        };

        let editor_role = Role {
            id: Uuid::now_v7(),
            name: "editor".to_string(),
            description: "Content editor".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            system_role: false,
            permissions: ["content.read", "content.write", "content.publish"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        let user_role = Role {
            id: Uuid::now_v7(),
            name: "user".to_string(),
            description: "Regular user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            system_role: true,
            permissions: ["user.read", "content.read"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        self.roles.insert(admin_role.id, admin_role);
        self.roles.insert(editor_role.id, editor_role);
        self.roles.insert(user_role.id, user_role);
    }

    pub async fn create_test_user(&mut self) -> Uuid {
        let user = User {
            id: Uuid::now_v7(),
            email: format!("user{}@example.com", Uuid::now_v7()),
            username: format!("user_{}", Uuid::now_v7()),
            display_name: "Test User".to_string(),
            password_hash: "hashed_password".to_string(),
            status: UserStatus::Active,
            email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            failed_login_attempts: 0,
            account_locked_until: None,
            password_changed_at: Some(Utc::now()),
            two_factor_enabled: false,
            two_factor_secret: None,
            recovery_email: None,
            recovery_email_verified: false,
            phone_number: None,
            phone_verified: false,
            profile_picture_url: None,
            bio: None,
            preferences: serde_json::json!({}),
            metadata: serde_json::json!({}),
        };
        let user_id = user.id;
        self.users.push(user);
        user_id
    }

    pub async fn create_role(
        &mut self,
        name: String,
        description: String,
        permissions: HashSet<String>,
        created_by: Uuid,
    ) -> Result<Uuid, AppError> {
        // Check if role name already exists
        if self.roles.values().any(|r| r.name == name) {
            return Err(AppError::Conflict {
                message: format!("Role '{}' already exists", name),
            });
        }

        // Verify all permissions exist
        let available_permissions: HashSet<String> = self.permissions.values()
            .map(|p| p.name.clone())
            .collect();

        for permission in &permissions {
            if !available_permissions.contains(permission) {
                return Err(AppError::BadRequest {
                    message: format!("Permission '{}' does not exist", permission),
                });
            }
        }

        let role = Role {
            id: Uuid::now_v7(),
            name,
            description,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            system_role: false,
            permissions,
        };

        let role_id = role.id;
        self.roles.insert(role_id, role);
        Ok(role_id)
    }

    pub async fn assign_role(
        &mut self,
        user_id: Uuid,
        role_id: Uuid,
        assigned_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
        context: Option<String>,
    ) -> Result<(), AppError> {
        // Verify user exists
        if !self.users.iter().any(|u| u.id == user_id) {
            return Err(AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            });
        }

        // Verify role exists
        if !self.roles.contains_key(&role_id) {
            return Err(AppError::NotFound {
                resource: "role".to_string(),
                id: Some(role_id.to_string()),
            });
        }

        // Check if user already has this role in the same context
        if let Some(assignments) = self.role_assignments.get(&user_id) {
            if assignments.iter().any(|a| a.role_id == role_id && a.context == context) {
                return Err(AppError::Conflict {
                    message: "User already has this role assigned".to_string(),
                });
            }
        }

        let assignment = RoleAssignment {
            id: Uuid::now_v7(),
            user_id,
            role_id,
            assigned_by,
            assigned_at: Utc::now(),
            expires_at,
            context,
        };

        self.role_assignments.entry(user_id)
            .or_insert_with(Vec::new)
            .push(assignment);

        // Clear permission cache for user
        self.permission_cache.remove(&user_id);

        Ok(())
    }

    pub async fn revoke_role(
        &mut self,
        user_id: Uuid,
        role_id: Uuid,
        context: Option<String>,
    ) -> Result<(), AppError> {
        let assignments = self.role_assignments.get_mut(&user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "role_assignment".to_string(),
                id: Some(user_id.to_string()),
            })?;

        let initial_len = assignments.len();
        assignments.retain(|a| !(a.role_id == role_id && a.context == context));

        if assignments.len() == initial_len {
            return Err(AppError::NotFound {
                resource: "role_assignment".to_string(),
                id: Some(format!("{}:{:?}", role_id, context)),
            });
        }

        // Clear permission cache for user
        self.permission_cache.remove(&user_id);

        Ok(())
    }

    pub async fn grant_permission(
        &mut self,
        user_id: Uuid,
        permission: String,
        resource_id: Option<String>,
        granted_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
        conditions: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        // Verify user exists
        if !self.users.iter().any(|u| u.id == user_id) {
            return Err(AppError::NotFound {
                resource: "user".to_string(),
                id: Some(user_id.to_string()),
            });
        }

        // Verify permission exists
        if !self.permissions.values().any(|p| p.name == permission) {
            return Err(AppError::BadRequest {
                message: format!("Permission '{}' does not exist", permission),
            });
        }

        let grant = PermissionGrant {
            id: Uuid::now_v7(),
            user_id,
            permission,
            resource_id,
            granted_by,
            granted_at: Utc::now(),
            expires_at,
            conditions,
        };

        self.permission_grants.entry(user_id)
            .or_insert_with(Vec::new)
            .push(grant);

        // Clear permission cache for user
        self.permission_cache.remove(&user_id);

        Ok(())
    }

    pub async fn revoke_permission(
        &mut self,
        user_id: Uuid,
        permission: String,
        resource_id: Option<String>,
    ) -> Result<(), AppError> {
        let grants = self.permission_grants.get_mut(&user_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "permission_grant".to_string(),
                id: Some(user_id.to_string()),
            })?;

        let initial_len = grants.len();
        grants.retain(|g| !(g.permission == permission && g.resource_id == resource_id));

        if grants.len() == initial_len {
            return Err(AppError::NotFound {
                resource: "permission_grant".to_string(),
                id: Some(permission),
            });
        }

        // Clear permission cache for user
        self.permission_cache.remove(&user_id);

        Ok(())
    }

    pub async fn has_permission(
        &mut self,
        user_id: Uuid,
        permission: &str,
        resource_id: Option<&str>,
        context: Option<&str>,
    ) -> bool {
        // Check cached permissions first
        if let Some(cached_permissions) = self.permission_cache.get(&user_id) {
            if cached_permissions.contains(permission) {
                return true;
            }
        }

        let user_permissions = self.get_user_permissions(user_id, context).await;

        // Check direct permission grants
        if let Some(grants) = self.permission_grants.get(&user_id) {
            for grant in grants {
                if grant.permission == permission {
                    // Check if permission is expired
                    if let Some(expires_at) = grant.expires_at {
                        if Utc::now() > expires_at {
                            continue;
                        }
                    }

                    // Check resource match
                    if grant.resource_id.as_deref() == resource_id || grant.resource_id.is_none() {
                        return true;
                    }
                }
            }
        }

        user_permissions.contains(permission)
    }

    pub async fn get_user_permissions(
        &mut self,
        user_id: Uuid,
        context: Option<&str>,
    ) -> HashSet<String> {
        let mut permissions = HashSet::new();

        // Get permissions from roles
        if let Some(assignments) = self.role_assignments.get(&user_id) {
            for assignment in assignments {
                // Check if assignment is expired
                if let Some(expires_at) = assignment.expires_at {
                    if Utc::now() > expires_at {
                        continue;
                    }
                }

                // Check context match
                if assignment.context.as_deref() != context && assignment.context.is_some() {
                    continue;
                }

                if let Some(role) = self.roles.get(&assignment.role_id) {
                    permissions.extend(role.permissions.clone());

                    // Add inherited permissions from parent roles
                    if let Some(parent_roles) = self.role_hierarchies.get(&role.id) {
                        for parent_role_id in parent_roles {
                            if let Some(parent_role) = self.roles.get(parent_role_id) {
                                permissions.extend(parent_role.permissions.clone());
                            }
                        }
                    }
                }
            }
        }

        // Cache the permissions
        self.permission_cache.insert(user_id, permissions.clone());

        permissions
    }

    pub async fn get_user_roles(&self, user_id: Uuid, context: Option<&str>) -> Vec<Role> {
        let mut roles = Vec::new();

        if let Some(assignments) = self.role_assignments.get(&user_id) {
            for assignment in assignments {
                // Check if assignment is expired
                if let Some(expires_at) = assignment.expires_at {
                    if Utc::now() > expires_at {
                        continue;
                    }
                }

                // Check context match
                if assignment.context.as_deref() != context && assignment.context.is_some() {
                    continue;
                }

                if let Some(role) = self.roles.get(&assignment.role_id) {
                    roles.push(role.clone());
                }
            }
        }

        roles
    }

    pub async fn add_role_hierarchy(
        &mut self,
        child_role_id: Uuid,
        parent_role_id: Uuid,
    ) -> Result<(), AppError> {
        // Verify both roles exist
        if !self.roles.contains_key(&child_role_id) {
            return Err(AppError::NotFound {
                resource: "role".to_string(),
                id: Some(child_role_id.to_string()),
            });
        }

        if !self.roles.contains_key(&parent_role_id) {
            return Err(AppError::NotFound {
                resource: "role".to_string(),
                id: Some(parent_role_id.to_string()),
            });
        }

        // Check for circular dependencies
        if self.would_create_cycle(child_role_id, parent_role_id) {
            return Err(AppError::BadRequest {
                message: "Adding this hierarchy would create a circular dependency".to_string(),
            });
        }

        self.role_hierarchies.entry(child_role_id)
            .or_insert_with(Vec::new)
            .push(parent_role_id);

        // Clear all permission caches since role hierarchy changed
        self.permission_cache.clear();

        Ok(())
    }

    fn would_create_cycle(&self, child_role_id: Uuid, parent_role_id: Uuid) -> bool {
        let mut visited = HashSet::new();
        self.check_cycle(parent_role_id, child_role_id, &mut visited)
    }

    fn check_cycle(&self, current_role: Uuid, target_role: Uuid, visited: &mut HashSet<Uuid>) -> bool {
        if current_role == target_role {
            return true;
        }

        if visited.contains(&current_role) {
            return false;
        }

        visited.insert(current_role);

        if let Some(parents) = self.role_hierarchies.get(&current_role) {
            for parent in parents {
                if self.check_cycle(*parent, target_role, visited) {
                    return true;
                }
            }
        }

        false
    }

    pub async fn cleanup_expired_assignments(&mut self) -> usize {
        let now = Utc::now();
        let mut count = 0;

        // Clean up expired role assignments
        for assignments in self.role_assignments.values_mut() {
            let initial_len = assignments.len();
            assignments.retain(|a| {
                if let Some(expires_at) = a.expires_at {
                    expires_at > now
                } else {
                    true
                }
            });
            count += initial_len - assignments.len();
        }

        // Clean up expired permission grants
        for grants in self.permission_grants.values_mut() {
            let initial_len = grants.len();
            grants.retain(|g| {
                if let Some(expires_at) = g.expires_at {
                    expires_at > now
                } else {
                    true
                }
            });
            count += initial_len - grants.len();
        }

        // Clear permission cache if anything was cleaned up
        if count > 0 {
            self.permission_cache.clear();
        }

        count
    }

    pub async fn get_role_by_name(&self, name: &str) -> Option<&Role> {
        self.roles.values().find(|r| r.name == name)
    }

    pub async fn update_role_permissions(
        &mut self,
        role_id: Uuid,
        permissions: HashSet<String>,
    ) -> Result<(), AppError> {
        let role = self.roles.get_mut(&role_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "role".to_string(),
                id: Some(role_id.to_string()),
            })?;

        if role.system_role {
            return Err(AppError::BadRequest {
                message: "Cannot modify system roles".to_string(),
            });
        }

        // Verify all permissions exist
        let available_permissions: HashSet<String> = self.permissions.values()
            .map(|p| p.name.clone())
            .collect();

        for permission in &permissions {
            if !available_permissions.contains(permission) {
                return Err(AppError::BadRequest {
                    message: format!("Permission '{}' does not exist", permission),
                });
            }
        }

        role.permissions = permissions;
        role.updated_at = Utc::now();

        // Clear permission cache since role permissions changed
        self.permission_cache.clear();

        Ok(())
    }

    pub async fn delete_role(&mut self, role_id: Uuid) -> Result<(), AppError> {
        let role = self.roles.get(&role_id)
            .ok_or_else(|| AppError::NotFound {
                resource: "role".to_string(),
                id: Some(role_id.to_string()),
            })?;

        if role.system_role {
            return Err(AppError::BadRequest {
                message: "Cannot delete system roles".to_string(),
            });
        }

        // Check if role is assigned to any users
        let has_assignments = self.role_assignments.values()
            .any(|assignments| assignments.iter().any(|a| a.role_id == role_id));

        if has_assignments {
            return Err(AppError::BadRequest {
                message: "Cannot delete role that is assigned to users".to_string(),
            });
        }

        self.roles.remove(&role_id);
        self.role_hierarchies.remove(&role_id);

        // Remove from parent hierarchies
        for parents in self.role_hierarchies.values_mut() {
            parents.retain(|&id| id != role_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_role() {
        let mut simulator = RolePermissionSimulator::new();
        let admin_id = simulator.create_test_user().await;

        let permissions = ["user.read", "user.write"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let result = simulator.create_role(
            "moderator".to_string(),
            "Content moderator".to_string(),
            permissions,
            admin_id,
        ).await;

        assert!(result.is_ok());
        let role_id = result.unwrap();

        let role = simulator.roles.get(&role_id).unwrap();
        assert_eq!(role.name, "moderator");
        assert_eq!(role.permissions.len(), 2);
        assert!(role.permissions.contains("user.read"));
        assert!(role.permissions.contains("user.write"));
    }

    #[tokio::test]
    async fn test_assign_role() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        let result = simulator.assign_role(
            user_id,
            role_id,
            admin_id,
            None,
            None,
        ).await;

        assert!(result.is_ok());

        let assignments = simulator.role_assignments.get(&user_id).unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].role_id, role_id);
    }

    #[tokio::test]
    async fn test_revoke_role() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        // Assign role
        simulator.assign_role(user_id, role_id, admin_id, None, None).await.unwrap();

        // Revoke role
        let result = simulator.revoke_role(user_id, role_id, None).await;
        assert!(result.is_ok());

        let assignments = simulator.role_assignments.get(&user_id);
        assert!(assignments.is_none() || assignments.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_grant_permission() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let result = simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            Some("resource123".to_string()),
            admin_id,
            None,
            None,
        ).await;

        assert!(result.is_ok());

        let grants = simulator.permission_grants.get(&user_id).unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].permission, "user.write");
        assert_eq!(grants[0].resource_id, Some("resource123".to_string()));
    }

    #[tokio::test]
    async fn test_has_permission_from_role() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        // Assign admin role
        simulator.assign_role(user_id, role_id, admin_id, None, None).await.unwrap();

        // Check permission
        let has_permission = simulator.has_permission(
            user_id,
            "user.write",
            None,
            None,
        ).await;

        assert!(has_permission);
    }

    #[tokio::test]
    async fn test_has_permission_from_direct_grant() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Grant specific permission
        simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            None,
            admin_id,
            None,
            None,
        ).await.unwrap();

        // Check permission
        let has_permission = simulator.has_permission(
            user_id,
            "user.write",
            None,
            None,
        ).await;

        assert!(has_permission);
    }

    #[tokio::test]
    async fn test_permission_expiry() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Grant permission that expires in the past
        let expired_time = Utc::now() - Duration::hours(1);
        simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            None,
            admin_id,
            Some(expired_time),
            None,
        ).await.unwrap();

        // Check permission should be false due to expiry
        let has_permission = simulator.has_permission(
            user_id,
            "user.write",
            None,
            None,
        ).await;

        assert!(!has_permission);
    }

    #[tokio::test]
    async fn test_role_assignment_expiry() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        // Assign role that expires in the past
        let expired_time = Utc::now() - Duration::hours(1);
        simulator.assign_role(
            user_id,
            role_id,
            admin_id,
            Some(expired_time),
            None,
        ).await.unwrap();

        // Check permission should be false due to expired role
        let has_permission = simulator.has_permission(
            user_id,
            "admin.full",
            None,
            None,
        ).await;

        assert!(!has_permission);
    }

    #[tokio::test]
    async fn test_context_specific_roles() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let editor_role = simulator.get_role_by_name("editor").await.unwrap();
        let role_id = editor_role.id;

        // Assign role in specific context
        simulator.assign_role(
            user_id,
            role_id,
            admin_id,
            None,
            Some("project123".to_string()),
        ).await.unwrap();

        let roles_in_context = simulator.get_user_roles(user_id, Some("project123")).await;
        assert_eq!(roles_in_context.len(), 1);
        assert_eq!(roles_in_context[0].name, "editor");

        let roles_global = simulator.get_user_roles(user_id, None).await;
        assert_eq!(roles_global.len(), 0);
    }

    #[tokio::test]
    async fn test_role_hierarchy() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Create a junior role
        let junior_permissions = ["content.read"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let junior_role_id = simulator.create_role(
            "junior".to_string(),
            "Junior role".to_string(),
            junior_permissions,
            admin_id,
        ).await.unwrap();

        let editor_role = simulator.get_role_by_name("editor").await.unwrap();
        let editor_role_id = editor_role.id;

        // Add hierarchy: junior inherits from editor
        simulator.add_role_hierarchy(junior_role_id, editor_role_id).await.unwrap();

        // Assign junior role to user
        simulator.assign_role(user_id, junior_role_id, admin_id, None, None).await.unwrap();

        // Should have permissions from both junior and editor roles
        let has_junior_permission = simulator.has_permission(
            user_id,
            "content.read",
            None,
            None,
        ).await;

        let has_inherited_permission = simulator.has_permission(
            user_id,
            "content.write",
            None,
            None,
        ).await;

        assert!(has_junior_permission);
        assert!(has_inherited_permission);
    }

    #[tokio::test]
    async fn test_prevent_circular_hierarchy() {
        let mut simulator = RolePermissionSimulator::new();
        let admin_id = simulator.create_test_user().await;

        let role1_id = simulator.create_role(
            "role1".to_string(),
            "Role 1".to_string(),
            HashSet::new(),
            admin_id,
        ).await.unwrap();

        let role2_id = simulator.create_role(
            "role2".to_string(),
            "Role 2".to_string(),
            HashSet::new(),
            admin_id,
        ).await.unwrap();

        // Add hierarchy: role1 -> role2
        simulator.add_role_hierarchy(role1_id, role2_id).await.unwrap();

        // Try to add reverse hierarchy: role2 -> role1 (should fail)
        let result = simulator.add_role_hierarchy(role2_id, role1_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest { .. }));
    }

    #[tokio::test]
    async fn test_cleanup_expired_assignments() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        // Create expired role assignment
        let expired_time = Utc::now() - Duration::hours(1);
        simulator.assign_role(
            user_id,
            role_id,
            admin_id,
            Some(expired_time),
            None,
        ).await.unwrap();

        // Create expired permission grant
        simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            None,
            admin_id,
            Some(expired_time),
            None,
        ).await.unwrap();

        // Clean up expired assignments
        let cleaned_count = simulator.cleanup_expired_assignments().await;
        assert_eq!(cleaned_count, 2);

        // Verify assignments were removed
        let assignments = simulator.role_assignments.get(&user_id);
        assert!(assignments.is_none() || assignments.unwrap().is_empty());

        let grants = simulator.permission_grants.get(&user_id);
        assert!(grants.is_none() || grants.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_update_role_permissions() {
        let mut simulator = RolePermissionSimulator::new();
        let admin_id = simulator.create_test_user().await;

        // Create a custom role
        let initial_permissions = ["user.read"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let role_id = simulator.create_role(
            "custom".to_string(),
            "Custom role".to_string(),
            initial_permissions,
            admin_id,
        ).await.unwrap();

        // Update permissions
        let new_permissions = ["user.read", "user.write"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let result = simulator.update_role_permissions(role_id, new_permissions).await;
        assert!(result.is_ok());

        let role = simulator.roles.get(&role_id).unwrap();
        assert_eq!(role.permissions.len(), 2);
        assert!(role.permissions.contains("user.read"));
        assert!(role.permissions.contains("user.write"));
    }

    #[tokio::test]
    async fn test_cannot_modify_system_role() {
        let mut simulator = RolePermissionSimulator::new();

        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        let role_id = admin_role.id;

        // Try to update system role (should fail)
        let new_permissions = ["user.read"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let result = simulator.update_role_permissions(role_id, new_permissions).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest { .. }));
    }

    #[tokio::test]
    async fn test_delete_role() {
        let mut simulator = RolePermissionSimulator::new();
        let admin_id = simulator.create_test_user().await;

        // Create a custom role
        let role_id = simulator.create_role(
            "deletable".to_string(),
            "Deletable role".to_string(),
            HashSet::new(),
            admin_id,
        ).await.unwrap();

        // Delete the role
        let result = simulator.delete_role(role_id).await;
        assert!(result.is_ok());

        // Verify role is deleted
        assert!(simulator.roles.get(&role_id).is_none());
    }

    #[tokio::test]
    async fn test_cannot_delete_assigned_role() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Create and assign a role
        let role_id = simulator.create_role(
            "assigned".to_string(),
            "Assigned role".to_string(),
            HashSet::new(),
            admin_id,
        ).await.unwrap();

        simulator.assign_role(user_id, role_id, admin_id, None, None).await.unwrap();

        // Try to delete assigned role (should fail)
        let result = simulator.delete_role(role_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest { .. }));
    }
}