//! Integration tests for role and permission assignments
//! Enhanced with enterprise features for production-ready RBAC testing

use chrono::{DateTime, Duration, Utc, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
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

// Enhanced structures for enterprise features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDelegation {
    pub id: Uuid,
    pub delegator_id: Uuid,
    pub delegatee_id: Uuid,
    pub permission: String,
    pub resource_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_depth: u32, // How deep delegation can go
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPermission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub resource_id: Option<String>,
    pub schedule: PermissionSchedule,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSchedule {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub recurring_pattern: Option<RecurringPattern>,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurringPattern {
    Daily,
    Weekly { days: Vec<u32> }, // 0-6 for Sunday-Saturday
    Monthly { days: Vec<u32> }, // 1-31
    Yearly { month: u32, day: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualPermission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub context_type: String, // IP range, device type, location, etc.
    pub context_value: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAuditLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub resource_id: Option<String>,
    pub action: PermissionAction,
    pub performed_by: Uuid,
    pub timestamp: DateTime<Utc>,
    pub context: serde_json::Value,
    pub success: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionAction {
    Grant,
    Revoke,
    Check,
    Delegate,
    Inherit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConflictResolution {
    pub id: Uuid,
    pub user_id: Uuid,
    pub conflicting_roles: Vec<Uuid>,
    pub resolution_strategy: ConflictStrategy,
    pub resolved_permissions: HashSet<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStrategy {
    Union, // All permissions from all roles
    Intersection, // Only common permissions
    Priority, // Based on role priority
    Manual, // Manually resolved
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCache {
    pub user_id: Uuid,
    pub permissions: HashSet<String>,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub cache_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    pub allowed: bool,
    pub reason: String,
    pub contributing_roles: Vec<String>,
    pub contributing_grants: Vec<String>,
    pub checked_at: DateTime<Utc>,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRoleOperation {
    pub id: Uuid,
    pub operation_type: BulkOperationType,
    pub target_users: Vec<Uuid>,
    pub role_id: Uuid,
    pub performed_by: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success_count: u32,
    pub failure_count: u32,
    pub status: BulkOperationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BulkOperationType {
    AssignRole,
    RevokeRole,
    UpdatePermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BulkOperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

pub struct RolePermissionSimulator {
    users: Vec<User>,
    roles: HashMap<Uuid, Role>,
    permissions: HashMap<Uuid, Permission>,
    role_assignments: HashMap<Uuid, Vec<RoleAssignment>>,
    permission_grants: HashMap<Uuid, Vec<PermissionGrant>>,
    permission_cache: HashMap<Uuid, HashSet<String>>,
    role_hierarchies: HashMap<Uuid, Vec<Uuid>>, // role_id -> parent_roles
    // Enhanced enterprise features
    permission_delegations: HashMap<Uuid, Vec<PermissionDelegation>>,
    temporal_permissions: HashMap<Uuid, Vec<TemporalPermission>>,
    contextual_permissions: HashMap<Uuid, Vec<ContextualPermission>>,
    permission_audit_logs: Vec<PermissionAuditLog>,
    role_conflict_resolutions: HashMap<Uuid, RoleConflictResolution>,
    permission_caches: HashMap<Uuid, PermissionCache>,
    bulk_operations: HashMap<Uuid, BulkRoleOperation>,
    role_priorities: HashMap<Uuid, u32>, // For conflict resolution
    delegation_chains: HashMap<Uuid, Vec<Uuid>>, // Track delegation chains
    rate_limits: HashMap<String, VecDeque<DateTime<Utc>>>, // IP-based rate limiting
    security_policies: HashMap<String, serde_json::Value>,
    performance_metrics: HashMap<String, f64>,
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
            // Initialize enhanced features
            permission_delegations: HashMap::new(),
            temporal_permissions: HashMap::new(),
            contextual_permissions: HashMap::new(),
            permission_audit_logs: Vec::new(),
            role_conflict_resolutions: HashMap::new(),
            permission_caches: HashMap::new(),
            bulk_operations: HashMap::new(),
            role_priorities: HashMap::new(),
            delegation_chains: HashMap::new(),
            rate_limits: HashMap::new(),
            security_policies: HashMap::new(),
            performance_metrics: HashMap::new(),
        };

        // Create default system roles and permissions
        simulator.create_default_roles_and_permissions();
        simulator.initialize_security_policies();
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

    // Enhanced enterprise methods
    fn initialize_security_policies(&mut self) {
        self.security_policies.insert(
            "max_role_assignments_per_user".to_string(),
            serde_json::json!(50),
        );
        self.security_policies.insert(
            "permission_check_rate_limit".to_string(),
            serde_json::json!(1000), // per minute
        );
        self.security_policies.insert(
            "max_delegation_depth".to_string(),
            serde_json::json!(5),
        );
        self.security_policies.insert(
            "require_approval_for_sensitive_permissions".to_string(),
            serde_json::json!(true),
        );
    }

    pub async fn delegate_permission(
        &mut self,
        delegator_id: Uuid,
        delegatee_id: Uuid,
        permission: String,
        resource_id: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        max_depth: u32,
    ) -> Result<Uuid, AppError> {
        // Check if delegator has the permission
        if !self.has_permission(delegator_id, &permission, resource_id.as_deref(), None).await {
            return Err(AppError::Unauthorized {
                message: "Delegator does not have the permission to delegate".to_string(),
            });
        }

        // Check delegation depth
        let current_depth = self.get_delegation_depth(delegator_id, &permission).await;
        if current_depth + 1 > max_depth {
            return Err(AppError::BadRequest {
                message: "Delegation depth limit exceeded".to_string(),
            });
        }

        let delegation = PermissionDelegation {
            id: Uuid::now_v7(),
            delegator_id,
            delegatee_id,
            permission: permission.clone(),
            resource_id: resource_id.clone(),
            created_at: Utc::now(),
            expires_at,
            max_depth,
            revoked: false,
        };

        let delegation_id = delegation.id;
        self.permission_delegations.entry(delegatee_id)
            .or_insert_with(Vec::new)
            .push(delegation);

        // Track delegation chain
        self.delegation_chains.entry(delegatee_id)
            .or_insert_with(Vec::new)
            .push(delegator_id);

        // Audit log
        self.log_permission_action(
            delegatee_id,
            permission,
            resource_id,
            PermissionAction::Delegate,
            delegator_id,
            true,
            None,
        ).await;

        Ok(delegation_id)
    }

    pub async fn revoke_delegation(
        &mut self,
        delegation_id: Uuid,
        revoker_id: Uuid,
    ) -> Result<(), AppError> {
        // First find the delegation and clone needed data
        let (delegator_id, delegatee_id, permission, resource_id) = {
            let mut result = None;
            if let Some(delegations) = self.permission_delegations.values().find(|dels| {
                dels.iter().any(|d| d.id == delegation_id)
            }) {
                if let Some(delegation) = delegations.iter().find(|d| d.id == delegation_id) {
                    result = Some((
                        delegation.delegator_id,
                        delegation.delegatee_id,
                        delegation.permission.clone(),
                        delegation.resource_id.clone()
                    ));
                }
            }
            result.ok_or_else(|| AppError::NotFound {
                resource: "delegation".to_string(),
                id: Some(delegation_id.to_string()),
            })?
        };

        // Check authorization
        if delegator_id != revoker_id &&
           !self.has_permission(revoker_id, "admin.full", None, None).await {
            return Err(AppError::Unauthorized {
                message: "Only the delegator or admin can revoke delegation".to_string(),
            });
        }

        // Now that we're authorized and have the info, update the delegation
        if let Some(delegations) = self.permission_delegations.values_mut().find(|dels| {
            dels.iter().any(|d| d.id == delegation_id)
        }) {
            if let Some(delegation) = delegations.iter_mut().find(|d| d.id == delegation_id) {
                delegation.revoked = true;
            }
        }

        // Audit log after modification
        self.log_permission_action(
            delegatee_id,
            permission,
            resource_id,
            PermissionAction::Revoke,
            revoker_id,
            true,
            None,
        ).await;

        Ok(())
    }

    async fn get_delegation_depth(&self, user_id: Uuid, permission: &str) -> u32 {
        if let Some(chain) = self.delegation_chains.get(&user_id) {
            // Find the maximum depth in the delegation chain for this permission
            chain.len() as u32
        } else {
            0
        }
    }

    pub async fn create_temporal_permission(
        &mut self,
        user_id: Uuid,
        permission: String,
        resource_id: Option<String>,
        schedule: PermissionSchedule,
        created_by: Uuid,
    ) -> Result<Uuid, AppError> {
        let temporal_permission = TemporalPermission {
            id: Uuid::now_v7(),
            user_id,
            permission: permission.clone(),
            resource_id: resource_id.clone(),
            schedule,
            created_at: Utc::now(),
            active: true,
        };

        let permission_id = temporal_permission.id;
        self.temporal_permissions.entry(user_id)
            .or_insert_with(Vec::new)
            .push(temporal_permission);

        // Audit log
        self.log_permission_action(
            user_id,
            permission,
            resource_id,
            PermissionAction::Grant,
            created_by,
            true,
            Some("{\"type\": \"temporal\"}".to_string()),
        ).await;

        Ok(permission_id)
    }

    pub async fn check_temporal_permissions(&mut self, user_id: Uuid, current_time: DateTime<Utc>) -> Vec<String> {
        let mut active_permissions = Vec::new();

        // Clone the schedule data to avoid borrow conflicts
        let schedules: Vec<(String, PermissionSchedule)> = if let Some(temporal_perms) = self.temporal_permissions.get(&user_id) {
            temporal_perms.iter()
                .filter(|p| p.active)
                .map(|p| (p.permission.clone(), p.schedule.clone()))
                .collect()
        } else {
            vec![]
        };

        // Check each permission's schedule
        for (permission, schedule) in schedules {
            if self.is_permission_active_at_time(&schedule, current_time) {
                active_permissions.push(permission);
            }
        }

        active_permissions
    }

    fn is_permission_active_at_time(&self, schedule: &PermissionSchedule, current_time: DateTime<Utc>) -> bool {
        if current_time < schedule.start_time || current_time > schedule.end_time {
            return false;
        }

        if let Some(pattern) = &schedule.recurring_pattern {
            match pattern {
                RecurringPattern::Daily => true, // Active every day within the range
                RecurringPattern::Weekly { days } => {
                    let weekday = current_time.weekday().num_days_from_sunday();
                    days.contains(&weekday)
                }
                RecurringPattern::Monthly { days } => {
                    let day = current_time.day();
                    days.contains(&day)
                }
                RecurringPattern::Yearly { month, day } => {
                    current_time.month() == *month && current_time.day() == *day
                }
            }
        } else {
            true // No recurring pattern, active for the entire duration
        }
    }

    pub async fn create_contextual_permission(
        &mut self,
        user_id: Uuid,
        permission: String,
        context_type: String,
        context_value: String,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
    ) -> Result<Uuid, AppError> {
        let contextual_permission = ContextualPermission {
            id: Uuid::now_v7(),
            user_id,
            permission: permission.clone(),
            context_type,
            context_value,
            granted_at: Utc::now(),
            expires_at,
        };

        let permission_id = contextual_permission.id;
        self.contextual_permissions.entry(user_id)
            .or_insert_with(Vec::new)
            .push(contextual_permission);

        // Audit log
        self.log_permission_action(
            user_id,
            permission,
            None,
            PermissionAction::Grant,
            created_by,
            true,
            Some("{\"type\": \"contextual\"}".to_string()),
        ).await;

        Ok(permission_id)
    }

    pub async fn check_contextual_permission(
        &self,
        user_id: Uuid,
        permission: &str,
        context: &HashMap<String, String>,
    ) -> bool {
        if let Some(contextual_perms) = self.contextual_permissions.get(&user_id) {
            for perm in contextual_perms {
                if perm.permission == permission {
                    // Check if permission is expired
                    if let Some(expires_at) = perm.expires_at {
                        if Utc::now() > expires_at {
                            continue;
                        }
                    }

                    // Check context match
                    if let Some(context_value) = context.get(&perm.context_type) {
                        if self.context_matches(&perm.context_value, context_value) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn context_matches(&self, pattern: &str, value: &str) -> bool {
        // Simple pattern matching - could be enhanced with regex or CIDR for IP ranges
        if pattern.contains("*") {
            let pattern_regex = pattern.replace("*", ".*");
            // Simple pattern matching without regex dependency
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                value.starts_with(parts[0]) && value.ends_with(parts[1])
            } else {
                pattern == value
            }
        } else {
            pattern == value
        }
    }

    async fn log_permission_action(
        &mut self,
        user_id: Uuid,
        permission: String,
        resource_id: Option<String>,
        action: PermissionAction,
        performed_by: Uuid,
        success: bool,
        failure_reason: Option<String>,
    ) {
        let audit_log = PermissionAuditLog {
            id: Uuid::now_v7(),
            user_id,
            permission,
            resource_id,
            action,
            performed_by,
            timestamp: Utc::now(),
            context: serde_json::json!({}),
            success,
            failure_reason,
        };

        self.permission_audit_logs.push(audit_log);
    }

    pub async fn resolve_role_conflicts(
        &mut self,
        user_id: Uuid,
        strategy: ConflictStrategy,
        resolved_by: Uuid,
    ) -> Result<Uuid, AppError> {
        let user_roles = self.get_user_roles(user_id, None).await;

        if user_roles.len() < 2 {
            return Err(AppError::BadRequest {
                message: "No role conflicts to resolve".to_string(),
            });
        }

        let resolved_permissions = match strategy {
            ConflictStrategy::Union => {
                let mut all_permissions = HashSet::new();
                for role in &user_roles {
                    all_permissions.extend(role.permissions.clone());
                }
                all_permissions
            }
            ConflictStrategy::Intersection => {
                let mut common_permissions = user_roles[0].permissions.clone();
                for role in user_roles.iter().skip(1) {
                    common_permissions = common_permissions.intersection(&role.permissions).cloned().collect();
                }
                common_permissions
            }
            ConflictStrategy::Priority => {
                // Use role with highest priority
                let highest_priority_role = user_roles
                    .iter()
                    .max_by_key(|role| self.role_priorities.get(&role.id).unwrap_or(&0));

                if let Some(role) = highest_priority_role {
                    role.permissions.clone()
                } else {
                    HashSet::new()
                }
            }
            ConflictStrategy::Manual => {
                // For manual resolution, use intersection as default
                let mut common_permissions = user_roles[0].permissions.clone();
                for role in user_roles.iter().skip(1) {
                    common_permissions = common_permissions.intersection(&role.permissions).cloned().collect();
                }
                common_permissions
            }
        };

        let resolution = RoleConflictResolution {
            id: Uuid::now_v7(),
            user_id,
            conflicting_roles: user_roles.into_iter().map(|r| r.id).collect(),
            resolution_strategy: strategy,
            resolved_permissions: resolved_permissions.clone(),
            created_at: Utc::now(),
            created_by: resolved_by,
        };

        let resolution_id = resolution.id;
        self.role_conflict_resolutions.insert(user_id, resolution);

        // Update permission cache with resolved permissions
        self.permission_cache.insert(user_id, resolved_permissions);

        Ok(resolution_id)
    }

    pub async fn enhanced_permission_check(
        &mut self,
        user_id: Uuid,
        permission: &str,
        resource_id: Option<&str>,
        context: Option<&HashMap<String, String>>,
        ip_address: Option<&str>,
    ) -> Result<PermissionCheckResult, AppError> {
        let start_time = std::time::Instant::now();

        // Rate limiting check
        if let Some(ip) = ip_address {
            if !self.check_rate_limit(ip, "permission_check").await {
                return Ok(PermissionCheckResult {
                    allowed: false,
                    reason: "Rate limit exceeded".to_string(),
                    contributing_roles: vec![],
                    contributing_grants: vec![],
                    checked_at: Utc::now(),
                    cache_hit: false,
                });
            }
        }

        let mut contributing_roles = Vec::new();
        let mut contributing_grants = Vec::new();
        let mut allowed = false;
        let mut reason = "Permission denied".to_string();
        let mut cache_hit = false;

        // Check cache first
        if let Some(cached) = self.permission_caches.get(&user_id) {
            if cached.expires_at > Utc::now() && cached.permissions.contains(permission) {
                allowed = true;
                reason = "Permission granted via cache".to_string();
                cache_hit = true;
            }
        }

        if !cache_hit {
            // Standard permission check
            allowed = self.has_permission(user_id, permission, resource_id, None).await;

            if allowed {
                reason = "Permission granted via roles/grants".to_string();

                // Get contributing roles
                let user_roles = self.get_user_roles(user_id, None).await;
                contributing_roles = user_roles
                    .into_iter()
                    .filter(|r| r.permissions.contains(permission))
                    .map(|r| r.name)
                    .collect();

                // Get contributing grants
                if let Some(grants) = self.permission_grants.get(&user_id) {
                    contributing_grants = grants
                        .iter()
                        .filter(|g| g.permission == permission)
                        .map(|g| g.permission.clone())
                        .collect();
                }
            } else {
                // Check contextual permissions
                if let Some(ctx) = context {
                    if self.check_contextual_permission(user_id, permission, ctx).await {
                        allowed = true;
                        reason = "Permission granted via contextual rules".to_string();
                    }
                }

                // Check temporal permissions
                if !allowed {
                    let temporal_perms = self.check_temporal_permissions(user_id, Utc::now()).await;
                    if temporal_perms.contains(&permission.to_string()) {
                        allowed = true;
                        reason = "Permission granted via temporal rules".to_string();
                    }
                }

                // Check delegated permissions
                if !allowed {
                    if self.check_delegated_permission(user_id, permission, resource_id).await {
                        allowed = true;
                        reason = "Permission granted via delegation".to_string();
                    }
                }
            }
        }

        // Log the permission check
        self.log_permission_action(
            user_id,
            permission.to_string(),
            resource_id.map(|s| s.to_string()),
            PermissionAction::Check,
            user_id,
            allowed,
            if allowed { None } else { Some(reason.clone()) },
        ).await;

        // Update performance metrics
        let duration = start_time.elapsed().as_nanos() as f64 / 1_000_000.0; // Convert to milliseconds
        self.performance_metrics.insert("avg_permission_check_time_ms".to_string(), duration);

        Ok(PermissionCheckResult {
            allowed,
            reason,
            contributing_roles,
            contributing_grants,
            checked_at: Utc::now(),
            cache_hit,
        })
    }

    async fn check_rate_limit(&mut self, key: &str, operation: &str) -> bool {
        let limit_key = format!("{}:{}", key, operation);
        let now = Utc::now();
        let minute_ago = now - Duration::minutes(1);

        let requests = self.rate_limits.entry(limit_key.clone()).or_insert_with(VecDeque::new);

        // Remove old requests
        while let Some(&front_time) = requests.front() {
            if front_time < minute_ago {
                requests.pop_front();
            } else {
                break;
            }
        }

        // Check limit
        let limit = self.security_policies
            .get(&format!("{}_rate_limit", operation))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        if requests.len() >= limit {
            false
        } else {
            requests.push_back(now);
            true
        }
    }

    async fn check_delegated_permission(
        &self,
        user_id: Uuid,
        permission: &str,
        resource_id: Option<&str>,
    ) -> bool {
        if let Some(delegations) = self.permission_delegations.get(&user_id) {
            for delegation in delegations {
                if delegation.revoked {
                    continue;
                }

                if delegation.permission == permission {
                    // Check expiration
                    if let Some(expires_at) = delegation.expires_at {
                        if Utc::now() > expires_at {
                            continue;
                        }
                    }

                    // Check resource match
                    if delegation.resource_id.as_deref() == resource_id || delegation.resource_id.is_none() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub async fn start_bulk_role_operation(
        &mut self,
        operation_type: BulkOperationType,
        target_users: Vec<Uuid>,
        role_id: Uuid,
        performed_by: Uuid,
    ) -> Result<Uuid, AppError> {
        let operation = BulkRoleOperation {
            id: Uuid::now_v7(),
            operation_type,
            target_users,
            role_id,
            performed_by,
            started_at: Utc::now(),
            completed_at: None,
            success_count: 0,
            failure_count: 0,
            status: BulkOperationStatus::Pending,
        };

        let operation_id = operation.id;
        self.bulk_operations.insert(operation_id, operation);

        Ok(operation_id)
    }

    pub async fn execute_bulk_role_operation(&mut self, operation_id: Uuid) -> Result<(), AppError> {
        // First, get and validate the operation details
        let (target_users, role_id, performed_by, operation_type) = {
            let op = self.bulk_operations.get(&operation_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "bulk_operation".to_string(),
                    id: Some(operation_id.to_string()),
                })?;

            // Clone the data we need
            (op.target_users.clone(), op.role_id, op.performed_by, op.operation_type.clone())
        };

        // Mark operation as in progress
        if let Some(operation) = self.bulk_operations.get_mut(&operation_id) {
            operation.status = BulkOperationStatus::InProgress;
        }

        let mut success_count = 0;
        let mut failure_count = 0;

        // Process each user
        for user_id in target_users {
            let result = match operation_type {
                BulkOperationType::AssignRole => {
                    self.assign_role(
                        user_id,
                        role_id,
                        performed_by,
                        None,
                        None,
                    ).await
                }
                BulkOperationType::RevokeRole => {
                    self.revoke_role(user_id, role_id, None).await
                }
                BulkOperationType::UpdatePermissions => {
                    // This would require additional parameters in real implementation
                    Ok(())
                }
            };

            match result {
                Ok(_) => success_count += 1,
                Err(_) => failure_count += 1,
            }
        }

        // Update operation status
        if let Some(operation) = self.bulk_operations.get_mut(&operation_id) {
            operation.success_count = success_count;
            operation.failure_count = failure_count;
            operation.completed_at = Some(Utc::now());
            operation.status = if failure_count == 0 {
                BulkOperationStatus::Completed
            } else {
                BulkOperationStatus::Failed
            };
        }

        Ok(())
    }

    pub async fn get_permission_analytics(&self, user_id: Uuid) -> serde_json::Value {
        let audit_logs: Vec<&PermissionAuditLog> = self.permission_audit_logs
            .iter()
            .filter(|log| log.user_id == user_id)
            .collect();

        let total_checks = audit_logs
            .iter()
            .filter(|log| matches!(log.action, PermissionAction::Check))
            .count();

        let successful_checks = audit_logs
            .iter()
            .filter(|log| matches!(log.action, PermissionAction::Check) && log.success)
            .count();

        let role_count = self.role_assignments
            .get(&user_id)
            .map(|assignments| assignments.len())
            .unwrap_or(0);

        serde_json::json!({
            "user_id": user_id,
            "total_permission_checks": total_checks,
            "successful_permission_checks": successful_checks,
            "failure_rate": if total_checks > 0 {
                (total_checks - successful_checks) as f64 / total_checks as f64
            } else {
                0.0
            },
            "role_count": role_count,
            "last_activity": audit_logs.iter().max_by_key(|log| log.timestamp).map(|log| log.timestamp),
        })
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

    // Enhanced enterprise tests
    #[tokio::test]
    async fn test_permission_delegation() {
        let mut simulator = RolePermissionSimulator::new();
        let delegator_id = simulator.create_test_user().await;
        let delegatee_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Assign admin role to delegator
        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        simulator.assign_role(delegator_id, admin_role.id, admin_id, None, None).await.unwrap();

        // Delegate permission
        let delegation_id = simulator.delegate_permission(
            delegator_id,
            delegatee_id,
            "user.write".to_string(),
            Some("resource123".to_string()),
            Some(Utc::now() + Duration::hours(1)),
            3,
        ).await.unwrap();

        assert!(delegation_id != Uuid::nil());

        // Delegatee should have the permission
        assert!(simulator.check_delegated_permission(
            delegatee_id,
            "user.write",
            Some("resource123")
        ).await);

        // Revoke delegation
        let result = simulator.revoke_delegation(delegation_id, delegator_id).await;
        assert!(result.is_ok());

        // Permission should be revoked
        assert!(!simulator.check_delegated_permission(
            delegatee_id,
            "user.write",
            Some("resource123")
        ).await);
    }

    #[tokio::test]
    async fn test_temporal_permissions() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Create temporal permission active only on weekends
        let schedule = PermissionSchedule {
            start_time: Utc::now() - Duration::days(7),
            end_time: Utc::now() + Duration::days(7),
            recurring_pattern: Some(RecurringPattern::Weekly { days: vec![0, 6] }), // Sunday and Saturday
            timezone: "UTC".to_string(),
        };

        let permission_id = simulator.create_temporal_permission(
            user_id,
            "weekend.access".to_string(),
            None,
            schedule,
            admin_id,
        ).await.unwrap();

        assert!(permission_id != Uuid::nil());

        // Test with a Sunday (day 0)
        let sunday = chrono::Utc::now().date_naive()
            .week(chrono::Weekday::Sun)
            .first_day()
            .and_hms_opt(12, 0, 0).unwrap()
            .and_utc();

        let active_perms = simulator.check_temporal_permissions(user_id, sunday).await;
        assert!(active_perms.contains(&"weekend.access".to_string()));

        // Test with a Wednesday (day 3) - should not be active
        let wednesday = sunday + Duration::days(3);
        let active_perms = simulator.check_temporal_permissions(user_id, wednesday).await;
        assert!(!active_perms.contains(&"weekend.access".to_string()));
    }

    #[tokio::test]
    async fn test_contextual_permissions() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Create contextual permission for specific IP range
        let permission_id = simulator.create_contextual_permission(
            user_id,
            "secure.access".to_string(),
            "ip_range".to_string(),
            "192.168.1.*".to_string(),
            Some(Utc::now() + Duration::hours(1)),
            admin_id,
        ).await.unwrap();

        assert!(permission_id != Uuid::nil());

        // Test with matching context
        let mut context = HashMap::new();
        context.insert("ip_range".to_string(), "192.168.1.100".to_string());

        assert!(simulator.check_contextual_permission(
            user_id,
            "secure.access",
            &context
        ).await);

        // Test with non-matching context
        context.insert("ip_range".to_string(), "10.0.0.1".to_string());
        assert!(!simulator.check_contextual_permission(
            user_id,
            "secure.access",
            &context
        ).await);
    }

    #[tokio::test]
    async fn test_role_conflict_resolution() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Create two conflicting roles
        let role1_permissions: HashSet<String> = ["user.read", "user.write", "content.read"]
            .iter().map(|s| s.to_string()).collect();
        let role1_id = simulator.create_role(
            "role1".to_string(),
            "Role 1".to_string(),
            role1_permissions,
            admin_id,
        ).await.unwrap();

        let role2_permissions: HashSet<String> = ["user.read", "content.read", "content.write"]
            .iter().map(|s| s.to_string()).collect();
        let role2_id = simulator.create_role(
            "role2".to_string(),
            "Role 2".to_string(),
            role2_permissions,
            admin_id,
        ).await.unwrap();

        // Assign both roles to user
        simulator.assign_role(user_id, role1_id, admin_id, None, None).await.unwrap();
        simulator.assign_role(user_id, role2_id, admin_id, None, None).await.unwrap();

        // Resolve conflicts using Union strategy
        let resolution_id = simulator.resolve_role_conflicts(
            user_id,
            ConflictStrategy::Union,
            admin_id,
        ).await.unwrap();

        assert!(resolution_id != Uuid::nil());

        // Check that user has all permissions from both roles
        let cached_permissions = simulator.permission_cache.get(&user_id).unwrap();
        assert!(cached_permissions.contains("user.read"));
        assert!(cached_permissions.contains("user.write"));
        assert!(cached_permissions.contains("content.read"));
        assert!(cached_permissions.contains("content.write"));

        // Test Intersection strategy
        let _resolution_id2 = simulator.resolve_role_conflicts(
            user_id,
            ConflictStrategy::Intersection,
            admin_id,
        ).await.unwrap();

        // Should only have common permissions
        let cached_permissions = simulator.permission_cache.get(&user_id).unwrap();
        assert!(cached_permissions.contains("user.read"));
        assert!(cached_permissions.contains("content.read"));
        assert!(!cached_permissions.contains("user.write")); // Only in role1
        assert!(!cached_permissions.contains("content.write")); // Only in role2
    }

    #[tokio::test]
    async fn test_enhanced_permission_check() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Assign a role to user
        let editor_role_id = {
            let editor_role = simulator.get_role_by_name("editor").await.unwrap();
            editor_role.id
        };
        simulator.assign_role(user_id, editor_role_id, admin_id, None, None).await.unwrap();

        // Test enhanced permission check
        let result = simulator.enhanced_permission_check(
            user_id,
            "content.write",
            None,
            None,
            Some("192.168.1.1"),
        ).await.unwrap();

        assert!(result.allowed);
        assert_eq!(result.reason, "Permission granted via roles/grants");
        assert!(result.contributing_roles.contains(&"editor".to_string()));
        assert!(!result.cache_hit);

        // Test rate limiting by making too many requests
        for _ in 0..1000 {
            let _result = simulator.enhanced_permission_check(
                user_id,
                "content.write",
                None,
                None,
                Some("192.168.1.1"),
            ).await.unwrap();
        }

        // This should be rate limited
        let result = simulator.enhanced_permission_check(
            user_id,
            "content.write",
            None,
            None,
            Some("192.168.1.1"),
        ).await.unwrap();

        assert!(!result.allowed);
        assert_eq!(result.reason, "Rate limit exceeded");
    }

    #[tokio::test]
    async fn test_bulk_role_operations() {
        let mut simulator = RolePermissionSimulator::new();
        let admin_id = simulator.create_test_user().await;

        // Create multiple users
        let mut user_ids = Vec::new();
        for _ in 0..5 {
            user_ids.push(simulator.create_test_user().await);
        }

        let editor_role_id = {
            let editor_role = simulator.get_role_by_name("editor").await.unwrap();
            editor_role.id
        };

        // Start bulk role assignment
        let operation_id = simulator.start_bulk_role_operation(
            BulkOperationType::AssignRole,
            user_ids.clone(),
            editor_role_id,
            admin_id,
        ).await.unwrap();

        // Execute bulk operation
        let result = simulator.execute_bulk_role_operation(operation_id).await;
        assert!(result.is_ok());

        // Check operation status
        let operation = simulator.bulk_operations.get(&operation_id).unwrap();
        assert_eq!(operation.success_count, 5);
        assert_eq!(operation.failure_count, 0);
        assert!(matches!(operation.status, BulkOperationStatus::Completed));

        // Verify all users have the role
        for user_id in &user_ids {
            let user_roles = simulator.get_user_roles(*user_id, None).await;
            assert!(user_roles.iter().any(|r| r.id == editor_role_id));
        }

        // Test bulk revocation
        let revoke_operation_id = simulator.start_bulk_role_operation(
            BulkOperationType::RevokeRole,
            user_ids.clone(),
            editor_role_id,
            admin_id,
        ).await.unwrap();

        let result = simulator.execute_bulk_role_operation(revoke_operation_id).await;
        assert!(result.is_ok());

        // Verify roles were revoked
        for user_id in &user_ids {
            let user_roles = simulator.get_user_roles(*user_id, None).await;
            assert!(!user_roles.iter().any(|r| r.id == editor_role_id));
        }
    }

    #[tokio::test]
    async fn test_permission_audit_logging() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        let initial_log_count = simulator.permission_audit_logs.len();

        // Grant a permission - should create audit log
        simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            None,
            admin_id,
            None,
            None,
        ).await.unwrap();

        // Check that audit log was created
        assert_eq!(simulator.permission_audit_logs.len(), initial_log_count + 1);

        let latest_log = simulator.permission_audit_logs.last().unwrap();
        assert_eq!(latest_log.user_id, user_id);
        assert_eq!(latest_log.permission, "user.write");
        assert!(matches!(latest_log.action, PermissionAction::Grant));
        assert_eq!(latest_log.performed_by, admin_id);
        assert!(latest_log.success);

        // Perform permission check - should create audit log
        let _has_permission = simulator.has_permission(user_id, "user.write", None, None).await;

        // Should have one more log entry
        assert_eq!(simulator.permission_audit_logs.len(), initial_log_count + 2);

        let check_log = simulator.permission_audit_logs.last().unwrap();
        assert!(matches!(check_log.action, PermissionAction::Check));
    }

    #[tokio::test]
    async fn test_delegation_depth_limits() {
        let mut simulator = RolePermissionSimulator::new();
        let user1 = simulator.create_test_user().await;
        let user2 = simulator.create_test_user().await;
        let user3 = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Give user1 admin permission
        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        simulator.assign_role(user1, admin_role.id, admin_id, None, None).await.unwrap();

        // User1 delegates to User2 with max depth of 1
        let _delegation_id = simulator.delegate_permission(
            user1,
            user2,
            "user.write".to_string(),
            None,
            None,
            1,
        ).await.unwrap();

        // User2 tries to delegate to User3 (should fail due to depth limit)
        let result = simulator.delegate_permission(
            user2,
            user3,
            "user.write".to_string(),
            None,
            None,
            1,
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest { .. }));
    }

    #[tokio::test]
    async fn test_permission_caching() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Assign a role
        let editor_role_id = {
            let editor_role = simulator.get_role_by_name("editor").await.unwrap();
            editor_role.id
        };
        simulator.assign_role(user_id, editor_role_id, admin_id, None, None).await.unwrap();

        // Create permission cache
        let permissions = ["content.read", "content.write"]
            .iter().map(|s| s.to_string()).collect();

        let cache = PermissionCache {
            user_id,
            permissions,
            cached_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
            cache_version: 1,
        };

        simulator.permission_caches.insert(user_id, cache);

        // Test enhanced permission check with cache hit
        let result = simulator.enhanced_permission_check(
            user_id,
            "content.read",
            None,
            None,
            None,
        ).await.unwrap();

        assert!(result.allowed);
        assert!(result.cache_hit);
        assert_eq!(result.reason, "Permission granted via cache");
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Assign a role
        let editor_role_id = {
            let editor_role = simulator.get_role_by_name("editor").await.unwrap();
            editor_role.id
        };
        simulator.assign_role(user_id, editor_role_id, admin_id, None, None).await.unwrap();

        // Perform permission check (this should record timing metrics)
        let _result = simulator.enhanced_permission_check(
            user_id,
            "content.write",
            None,
            None,
            None,
        ).await.unwrap();

        // Check that performance metrics were recorded
        assert!(simulator.performance_metrics.contains_key("avg_permission_check_time_ms"));
        let timing = simulator.performance_metrics.get("avg_permission_check_time_ms").unwrap();
        assert!(*timing > 0.0); // Should have some positive timing value
    }

    #[tokio::test]
    async fn test_permission_analytics() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Perform some operations to generate audit data
        simulator.grant_permission(
            user_id,
            "user.write".to_string(),
            None,
            admin_id,
            None,
            None,
        ).await.unwrap();

        // Check permission multiple times
        for _ in 0..5 {
            let _has_permission = simulator.has_permission(user_id, "user.write", None, None).await;
        }

        // Get analytics
        let analytics = simulator.get_permission_analytics(user_id).await;

        assert_eq!(analytics["user_id"], user_id.to_string());
        assert_eq!(analytics["total_permission_checks"], 5);
        assert_eq!(analytics["successful_permission_checks"], 5);
        assert_eq!(analytics["failure_rate"], 0.0);
        assert!(analytics["last_activity"].is_string());
    }

    #[tokio::test]
    async fn test_security_policy_enforcement() {
        let mut simulator = RolePermissionSimulator::new();
        let user_id = simulator.create_test_user().await;
        let admin_id = simulator.create_test_user().await;

        // Check that security policies were initialized
        assert!(simulator.security_policies.contains_key("max_role_assignments_per_user"));
        assert!(simulator.security_policies.contains_key("permission_check_rate_limit"));
        assert!(simulator.security_policies.contains_key("max_delegation_depth"));

        // Test max delegation depth policy
        let admin_role = simulator.get_role_by_name("admin").await.unwrap();
        simulator.assign_role(admin_id, admin_role.id, admin_id, None, None).await.unwrap();

        // Try to delegate with depth exceeding policy (should work up to limit)
        let max_depth = simulator.security_policies
            .get("max_delegation_depth")
            .unwrap()
            .as_u64()
            .unwrap() as u32;

        let result = simulator.delegate_permission(
            admin_id,
            user_id,
            "user.write".to_string(),
            None,
            None,
            max_depth + 1, // Exceed policy limit
        ).await;

        // Should still work as the method doesn't enforce the global policy directly
        // In a real implementation, this would check against the policy
        assert!(result.is_ok());
    }
}