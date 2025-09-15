//! Admin User Management Tests
//!
//! This module contains comprehensive tests for admin user management operations.
//! Tests include user creation, deletion, role management, permission changes,
//! bulk operations, audit logging, and admin-specific features.

#[cfg(test)]
mod admin_management_tests {
    use chrono::{DateTime, Utc, Duration};
    use uuid::Uuid;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    // Admin management simulator for testing
    #[derive(Debug, Clone)]
    pub struct AdminManagementSimulator {
        users: Arc<Mutex<HashMap<Uuid, ManagedUser>>>,
        roles: Arc<Mutex<HashMap<String, Role>>>,
        permissions: Arc<Mutex<HashMap<String, Permission>>>,
        audit_log: Arc<Mutex<Vec<AuditEntry>>>,
        admin_sessions: Arc<Mutex<HashMap<String, AdminSession>>>,
        bulk_operations: Arc<Mutex<Vec<BulkOperation>>>,
        system_settings: Arc<Mutex<SystemSettings>>,
    }

    #[derive(Debug, Clone)]
    pub struct ManagedUser {
        pub id: Uuid,
        pub email: String,
        pub username: String,
        pub status: UserStatus,
        pub roles: HashSet<String>,
        pub permissions: HashSet<String>,
        pub created_at: DateTime<Utc>,
        pub created_by: Uuid,
        pub last_modified: DateTime<Utc>,
        pub modified_by: Option<Uuid>,
        pub locked_until: Option<DateTime<Utc>>,
        pub failed_login_attempts: u32,
        pub email_verified: bool,
        pub two_factor_enabled: bool,
        pub metadata: HashMap<String, String>,
    }

    #[derive(Debug, Clone)]
    pub struct Role {
        pub name: String,
        pub description: String,
        pub permissions: HashSet<String>,
        pub priority: u32,
        pub is_system_role: bool,
        pub created_at: DateTime<Utc>,
        pub created_by: Uuid,
    }

    #[derive(Debug, Clone)]
    pub struct Permission {
        pub name: String,
        pub resource: String,
        pub action: String,
        pub description: String,
        pub is_system_permission: bool,
    }

    #[derive(Debug, Clone)]
    pub struct AdminSession {
        pub id: String,
        pub admin_id: Uuid,
        pub started_at: DateTime<Utc>,
        pub last_activity: DateTime<Utc>,
        pub ip_address: String,
        pub actions_performed: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct AuditEntry {
        pub id: Uuid,
        pub timestamp: DateTime<Utc>,
        pub admin_id: Uuid,
        pub action: AdminAction,
        pub target_user_id: Option<Uuid>,
        pub details: HashMap<String, String>,
        pub ip_address: String,
        pub success: bool,
        pub error_message: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub enum AdminAction {
        CreateUser,
        DeleteUser,
        UpdateUser,
        AssignRole,
        RemoveRole,
        GrantPermission,
        RevokePermission,
        LockAccount,
        UnlockAccount,
        ResetPassword,
        ForceLogout,
        BulkOperation,
        UpdateSystemSettings,
        ViewAuditLog,
        ExportData,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum UserStatus {
        Active,
        Inactive,
        Locked,
        Suspended,
        Deleted,
        PendingVerification,
    }

    #[derive(Debug, Clone)]
    pub struct BulkOperation {
        pub id: Uuid,
        pub operation_type: BulkOperationType,
        pub target_users: Vec<Uuid>,
        pub initiated_by: Uuid,
        pub started_at: DateTime<Utc>,
        pub completed_at: Option<DateTime<Utc>>,
        pub success_count: u32,
        pub failure_count: u32,
        pub status: BulkOperationStatus,
    }

    #[derive(Debug, Clone)]
    pub enum BulkOperationType {
        AssignRole(String),
        RemoveRole(String),
        UpdateStatus(UserStatus),
        SendEmail(String),
        ExportData,
        DeleteUsers,
        ResetPasswords,
    }

    #[derive(Debug, Clone)]
    pub enum BulkOperationStatus {
        Pending,
        InProgress,
        Completed,
        Failed,
        PartiallyCompleted,
    }

    #[derive(Debug, Clone)]
    pub struct SystemSettings {
        pub max_login_attempts: u32,
        pub lockout_duration_minutes: u32,
        pub password_expiry_days: u32,
        pub require_two_factor: bool,
        pub audit_retention_days: u32,
        pub allow_self_registration: bool,
        pub default_user_role: String,
    }

    impl AdminManagementSimulator {
        pub fn new() -> Self {
            let mut roles = HashMap::new();

            // Create default roles
            roles.insert("admin".to_string(), Role {
                name: "admin".to_string(),
                description: "System Administrator".to_string(),
                permissions: HashSet::from([
                    "users.create".to_string(),
                    "users.read".to_string(),
                    "users.update".to_string(),
                    "users.delete".to_string(),
                    "roles.manage".to_string(),
                    "system.configure".to_string(),
                ]),
                priority: 100,
                is_system_role: true,
                created_at: Utc::now(),
                created_by: Uuid::nil(),
            });

            roles.insert("moderator".to_string(), Role {
                name: "moderator".to_string(),
                description: "Content Moderator".to_string(),
                permissions: HashSet::from([
                    "users.read".to_string(),
                    "users.update".to_string(),
                    "content.moderate".to_string(),
                ]),
                priority: 50,
                is_system_role: true,
                created_at: Utc::now(),
                created_by: Uuid::nil(),
            });

            roles.insert("user".to_string(), Role {
                name: "user".to_string(),
                description: "Regular User".to_string(),
                permissions: HashSet::from([
                    "profile.read".to_string(),
                    "profile.update".to_string(),
                ]),
                priority: 10,
                is_system_role: true,
                created_at: Utc::now(),
                created_by: Uuid::nil(),
            });

            let system_settings = SystemSettings {
                max_login_attempts: 5,
                lockout_duration_minutes: 30,
                password_expiry_days: 90,
                require_two_factor: false,
                audit_retention_days: 365,
                allow_self_registration: true,
                default_user_role: "user".to_string(),
            };

            Self {
                users: Arc::new(Mutex::new(HashMap::new())),
                roles: Arc::new(Mutex::new(roles)),
                permissions: Arc::new(Mutex::new(HashMap::new())),
                audit_log: Arc::new(Mutex::new(Vec::new())),
                admin_sessions: Arc::new(Mutex::new(HashMap::new())),
                bulk_operations: Arc::new(Mutex::new(Vec::new())),
                system_settings: Arc::new(Mutex::new(system_settings)),
            }
        }

        // Create admin user for testing
        pub fn create_admin_user(&self) -> (Uuid, AdminSession) {
            let admin_id = Uuid::now_v7();
            let admin = ManagedUser {
                id: admin_id,
                email: "admin@example.com".to_string(),
                username: "admin".to_string(),
                status: UserStatus::Active,
                roles: HashSet::from(["admin".to_string()]),
                permissions: HashSet::from([
                    "users.create".to_string(),
                    "users.read".to_string(),
                    "users.update".to_string(),
                    "users.delete".to_string(),
                    "roles.manage".to_string(),
                    "system.configure".to_string(),
                ]),
                created_at: Utc::now(),
                created_by: Uuid::nil(),
                last_modified: Utc::now(),
                modified_by: None,
                locked_until: None,
                failed_login_attempts: 0,
                email_verified: true,
                two_factor_enabled: true,
                metadata: HashMap::new(),
            };

            let session = AdminSession {
                id: format!("admin_session_{}", Uuid::now_v7()),
                admin_id,
                started_at: Utc::now(),
                last_activity: Utc::now(),
                ip_address: "192.168.1.100".to_string(),
                actions_performed: Vec::new(),
            };

            let mut users = self.users.lock().unwrap();
            users.insert(admin_id, admin);

            let mut sessions = self.admin_sessions.lock().unwrap();
            sessions.insert(session.id.clone(), session.clone());

            (admin_id, session)
        }

        // Admin creates a new user
        pub async fn admin_create_user(
            &self,
            admin_id: Uuid,
            email: &str,
            username: &str,
            role: &str,
        ) -> Result<Uuid, String> {
            // Verify admin permissions
            if !self.verify_admin_permission(admin_id, "users.create") {
                self.log_audit_entry(AuditEntry {
                    id: Uuid::now_v7(),
                    timestamp: Utc::now(),
                    admin_id,
                    action: AdminAction::CreateUser,
                    target_user_id: None,
                    details: HashMap::from([
                        ("email".to_string(), email.to_string()),
                        ("error".to_string(), "Permission denied".to_string()),
                    ]),
                    ip_address: "192.168.1.100".to_string(),
                    success: false,
                    error_message: Some("Permission denied".to_string()),
                });
                return Err("Permission denied".to_string());
            }

            let user_id = Uuid::now_v7();
            let user = ManagedUser {
                id: user_id,
                email: email.to_string(),
                username: username.to_string(),
                status: UserStatus::PendingVerification,
                roles: HashSet::from([role.to_string()]),
                permissions: self.get_role_permissions(role),
                created_at: Utc::now(),
                created_by: admin_id,
                last_modified: Utc::now(),
                modified_by: Some(admin_id),
                locked_until: None,
                failed_login_attempts: 0,
                email_verified: false,
                two_factor_enabled: false,
                metadata: HashMap::new(),
            };

            let mut users = self.users.lock().unwrap();
            users.insert(user_id, user);

            self.log_audit_entry(AuditEntry {
                id: Uuid::now_v7(),
                timestamp: Utc::now(),
                admin_id,
                action: AdminAction::CreateUser,
                target_user_id: Some(user_id),
                details: HashMap::from([
                    ("email".to_string(), email.to_string()),
                    ("username".to_string(), username.to_string()),
                    ("role".to_string(), role.to_string()),
                ]),
                ip_address: "192.168.1.100".to_string(),
                success: true,
                error_message: None,
            });

            Ok(user_id)
        }

        // Admin deletes a user
        pub async fn admin_delete_user(&self, admin_id: Uuid, user_id: Uuid) -> Result<(), String> {
            if !self.verify_admin_permission(admin_id, "users.delete") {
                return Err("Permission denied".to_string());
            }

            let mut users = self.users.lock().unwrap();
            if let Some(user) = users.get_mut(&user_id) {
                // Prevent deleting other admins
                if user.roles.contains("admin") && admin_id != user_id {
                    return Err("Cannot delete another admin".to_string());
                }

                user.status = UserStatus::Deleted;
                user.modified_by = Some(admin_id);
                user.last_modified = Utc::now();

                self.log_audit_entry(AuditEntry {
                    id: Uuid::now_v7(),
                    timestamp: Utc::now(),
                    admin_id,
                    action: AdminAction::DeleteUser,
                    target_user_id: Some(user_id),
                    details: HashMap::from([
                        ("action".to_string(), "soft_delete".to_string()),
                    ]),
                    ip_address: "192.168.1.100".to_string(),
                    success: true,
                    error_message: None,
                });

                Ok(())
            } else {
                Err("User not found".to_string())
            }
        }

        // Admin assigns role to user
        pub async fn admin_assign_role(
            &self,
            admin_id: Uuid,
            user_id: Uuid,
            role: &str,
        ) -> Result<(), String> {
            if !self.verify_admin_permission(admin_id, "roles.manage") {
                return Err("Permission denied".to_string());
            }

            let mut users = self.users.lock().unwrap();
            if let Some(user) = users.get_mut(&user_id) {
                let old_roles = user.roles.clone();
                user.roles.insert(role.to_string());
                user.permissions = self.get_combined_permissions(&user.roles);
                user.modified_by = Some(admin_id);
                user.last_modified = Utc::now();

                self.log_audit_entry(AuditEntry {
                    id: Uuid::now_v7(),
                    timestamp: Utc::now(),
                    admin_id,
                    action: AdminAction::AssignRole,
                    target_user_id: Some(user_id),
                    details: HashMap::from([
                        ("role".to_string(), role.to_string()),
                        ("old_roles".to_string(), format!("{:?}", old_roles)),
                        ("new_roles".to_string(), format!("{:?}", user.roles)),
                    ]),
                    ip_address: "192.168.1.100".to_string(),
                    success: true,
                    error_message: None,
                });

                Ok(())
            } else {
                Err("User not found".to_string())
            }
        }

        // Admin removes role from user
        pub async fn admin_remove_role(
            &self,
            admin_id: Uuid,
            user_id: Uuid,
            role: &str,
        ) -> Result<(), String> {
            if !self.verify_admin_permission(admin_id, "roles.manage") {
                return Err("Permission denied".to_string());
            }

            let mut users = self.users.lock().unwrap();

            // Check if we're removing the last admin role
            if role == "admin" {
                let user_has_admin = users.get(&user_id)
                    .map(|u| u.roles.contains("admin"))
                    .unwrap_or(false);

                if user_has_admin {
                    let admin_count = users.values()
                        .filter(|u| u.roles.contains("admin") && u.status == UserStatus::Active)
                        .count();
                    if admin_count <= 1 {
                        return Err("Cannot remove last admin role".to_string());
                    }
                }
            }

            if let Some(user) = users.get_mut(&user_id) {
                user.roles.remove(role);
                user.permissions = self.get_combined_permissions(&user.roles);
                user.modified_by = Some(admin_id);
                user.last_modified = Utc::now();

                self.log_audit_entry(AuditEntry {
                    id: Uuid::now_v7(),
                    timestamp: Utc::now(),
                    admin_id,
                    action: AdminAction::RemoveRole,
                    target_user_id: Some(user_id),
                    details: HashMap::from([
                        ("role".to_string(), role.to_string()),
                    ]),
                    ip_address: "192.168.1.100".to_string(),
                    success: true,
                    error_message: None,
                });

                Ok(())
            } else {
                Err("User not found".to_string())
            }
        }

        // Admin locks user account
        pub async fn admin_lock_account(
            &self,
            admin_id: Uuid,
            user_id: Uuid,
            duration_hours: u32,
        ) -> Result<(), String> {
            if !self.verify_admin_permission(admin_id, "users.update") {
                return Err("Permission denied".to_string());
            }

            let mut users = self.users.lock().unwrap();
            if let Some(user) = users.get_mut(&user_id) {
                user.status = UserStatus::Locked;
                user.locked_until = Some(Utc::now() + Duration::hours(duration_hours as i64));
                user.modified_by = Some(admin_id);
                user.last_modified = Utc::now();

                self.log_audit_entry(AuditEntry {
                    id: Uuid::now_v7(),
                    timestamp: Utc::now(),
                    admin_id,
                    action: AdminAction::LockAccount,
                    target_user_id: Some(user_id),
                    details: HashMap::from([
                        ("duration_hours".to_string(), duration_hours.to_string()),
                        ("locked_until".to_string(), user.locked_until.unwrap().to_string()),
                    ]),
                    ip_address: "192.168.1.100".to_string(),
                    success: true,
                    error_message: None,
                });

                Ok(())
            } else {
                Err("User not found".to_string())
            }
        }

        // Admin performs bulk operation
        pub async fn admin_bulk_operation(
            &self,
            admin_id: Uuid,
            operation_type: BulkOperationType,
            target_users: Vec<Uuid>,
        ) -> Result<Uuid, String> {
            if !self.verify_admin_permission(admin_id, "users.update") {
                return Err("Permission denied".to_string());
            }

            let operation_id = Uuid::now_v7();
            let mut operation = BulkOperation {
                id: operation_id,
                operation_type: operation_type.clone(),
                target_users: target_users.clone(),
                initiated_by: admin_id,
                started_at: Utc::now(),
                completed_at: None,
                success_count: 0,
                failure_count: 0,
                status: BulkOperationStatus::InProgress,
            };

            // Simulate bulk operation execution
            let mut users = self.users.lock().unwrap();
            for user_id in &target_users {
                if let Some(user) = users.get_mut(user_id) {
                    match &operation_type {
                        BulkOperationType::AssignRole(role) => {
                            user.roles.insert(role.clone());
                            user.permissions = self.get_combined_permissions(&user.roles);
                            operation.success_count += 1;
                        }
                        BulkOperationType::RemoveRole(role) => {
                            user.roles.remove(role);
                            user.permissions = self.get_combined_permissions(&user.roles);
                            operation.success_count += 1;
                        }
                        BulkOperationType::UpdateStatus(status) => {
                            user.status = status.clone();
                            operation.success_count += 1;
                        }
                        _ => {
                            operation.success_count += 1; // Simulate other operations
                        }
                    }
                    user.modified_by = Some(admin_id);
                    user.last_modified = Utc::now();
                } else {
                    operation.failure_count += 1;
                }
            }

            operation.completed_at = Some(Utc::now());
            operation.status = if operation.failure_count == 0 {
                BulkOperationStatus::Completed
            } else if operation.success_count == 0 {
                BulkOperationStatus::Failed
            } else {
                BulkOperationStatus::PartiallyCompleted
            };

            let mut bulk_operations = self.bulk_operations.lock().unwrap();
            bulk_operations.push(operation.clone());

            self.log_audit_entry(AuditEntry {
                id: Uuid::now_v7(),
                timestamp: Utc::now(),
                admin_id,
                action: AdminAction::BulkOperation,
                target_user_id: None,
                details: HashMap::from([
                    ("operation_id".to_string(), operation_id.to_string()),
                    ("operation_type".to_string(), format!("{:?}", operation_type)),
                    ("target_count".to_string(), target_users.len().to_string()),
                    ("success_count".to_string(), operation.success_count.to_string()),
                    ("failure_count".to_string(), operation.failure_count.to_string()),
                ]),
                ip_address: "192.168.1.100".to_string(),
                success: true,
                error_message: None,
            });

            Ok(operation_id)
        }

        // Get audit log entries
        pub fn get_audit_log(&self, admin_id: Uuid, limit: usize) -> Result<Vec<AuditEntry>, String> {
            if !self.verify_admin_permission(admin_id, "system.configure") {
                return Err("Permission denied".to_string());
            }

            let audit_log = self.audit_log.lock().unwrap();
            let entries: Vec<AuditEntry> = audit_log.iter()
                .rev()
                .take(limit)
                .cloned()
                .collect();

            Ok(entries)
        }

        // Update system settings
        pub async fn update_system_settings(
            &self,
            admin_id: Uuid,
            settings: SystemSettings,
        ) -> Result<(), String> {
            if !self.verify_admin_permission(admin_id, "system.configure") {
                return Err("Permission denied".to_string());
            }

            let mut system_settings = self.system_settings.lock().unwrap();
            *system_settings = settings.clone();

            self.log_audit_entry(AuditEntry {
                id: Uuid::now_v7(),
                timestamp: Utc::now(),
                admin_id,
                action: AdminAction::UpdateSystemSettings,
                target_user_id: None,
                details: HashMap::from([
                    ("max_login_attempts".to_string(), settings.max_login_attempts.to_string()),
                    ("lockout_duration_minutes".to_string(), settings.lockout_duration_minutes.to_string()),
                    ("password_expiry_days".to_string(), settings.password_expiry_days.to_string()),
                    ("require_two_factor".to_string(), settings.require_two_factor.to_string()),
                ]),
                ip_address: "192.168.1.100".to_string(),
                success: true,
                error_message: None,
            });

            Ok(())
        }

        // Get user statistics for admin dashboard
        pub fn get_user_statistics(&self, admin_id: Uuid) -> Result<HashMap<String, usize>, String> {
            if !self.verify_admin_permission(admin_id, "users.read") {
                return Err("Permission denied".to_string());
            }

            let users = self.users.lock().unwrap();
            let mut stats = HashMap::new();

            stats.insert("total_users".to_string(), users.len());
            stats.insert("active_users".to_string(),
                users.values().filter(|u| u.status == UserStatus::Active).count());
            stats.insert("locked_users".to_string(),
                users.values().filter(|u| u.status == UserStatus::Locked).count());
            stats.insert("pending_verification".to_string(),
                users.values().filter(|u| u.status == UserStatus::PendingVerification).count());
            stats.insert("admin_count".to_string(),
                users.values().filter(|u| u.roles.contains("admin")).count());
            stats.insert("two_factor_enabled".to_string(),
                users.values().filter(|u| u.two_factor_enabled).count());

            Ok(stats)
        }

        // Helper: Verify admin permission
        fn verify_admin_permission(&self, admin_id: Uuid, permission: &str) -> bool {
            let users = self.users.lock().unwrap();
            if let Some(admin) = users.get(&admin_id) {
                admin.permissions.contains(permission)
            } else {
                false
            }
        }

        // Helper: Get role permissions
        fn get_role_permissions(&self, role_name: &str) -> HashSet<String> {
            let roles = self.roles.lock().unwrap();
            if let Some(role) = roles.get(role_name) {
                role.permissions.clone()
            } else {
                HashSet::new()
            }
        }

        // Helper: Get combined permissions for multiple roles
        fn get_combined_permissions(&self, role_names: &HashSet<String>) -> HashSet<String> {
            let roles = self.roles.lock().unwrap();
            let mut permissions = HashSet::new();
            for role_name in role_names {
                if let Some(role) = roles.get(role_name) {
                    permissions.extend(role.permissions.clone());
                }
            }
            permissions
        }

        // Helper: Log audit entry
        fn log_audit_entry(&self, entry: AuditEntry) {
            let mut audit_log = self.audit_log.lock().unwrap();
            audit_log.push(entry);
        }
    }

    // Test admin user creation
    #[tokio::test]
    async fn test_admin_create_user() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        let result = simulator.admin_create_user(
            admin_id,
            "newuser@example.com",
            "newuser",
            "user"
        ).await;

        assert!(result.is_ok(), "Admin should be able to create user");
        let user_id = result.unwrap();

        let users = simulator.users.lock().unwrap();
        let user = users.get(&user_id).unwrap();
        assert_eq!(user.email, "newuser@example.com");
        assert_eq!(user.username, "newuser");
        assert!(user.roles.contains("user"));
        assert_eq!(user.created_by, admin_id);
    }

    // Test admin user deletion
    #[tokio::test]
    async fn test_admin_delete_user() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // Create a user to delete
        let user_id = simulator.admin_create_user(
            admin_id,
            "deleteme@example.com",
            "deleteme",
            "user"
        ).await.unwrap();

        // Delete the user
        let result = simulator.admin_delete_user(admin_id, user_id).await;
        assert!(result.is_ok(), "Admin should be able to delete user");

        let users = simulator.users.lock().unwrap();
        let user = users.get(&user_id).unwrap();
        assert!(matches!(user.status, UserStatus::Deleted));
    }

    // Test role assignment
    #[tokio::test]
    async fn test_admin_role_assignment() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        let user_id = simulator.admin_create_user(
            admin_id,
            "roletest@example.com",
            "roletest",
            "user"
        ).await.unwrap();

        // Assign moderator role
        let result = simulator.admin_assign_role(admin_id, user_id, "moderator").await;
        assert!(result.is_ok(), "Admin should be able to assign role");

        let users = simulator.users.lock().unwrap();
        let user = users.get(&user_id).unwrap();
        assert!(user.roles.contains("moderator"));
        assert!(user.roles.contains("user")); // Should still have original role
    }

    // Test role removal
    #[tokio::test]
    async fn test_admin_role_removal() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        let user_id = simulator.admin_create_user(
            admin_id,
            "roleremove@example.com",
            "roleremove",
            "user"
        ).await.unwrap();

        // Assign then remove role
        simulator.admin_assign_role(admin_id, user_id, "moderator").await.unwrap();
        let result = simulator.admin_remove_role(admin_id, user_id, "moderator").await;
        assert!(result.is_ok(), "Admin should be able to remove role");

        let users = simulator.users.lock().unwrap();
        let user = users.get(&user_id).unwrap();
        assert!(!user.roles.contains("moderator"));
        assert!(user.roles.contains("user")); // Should still have user role
    }

    // Test account locking
    #[tokio::test]
    async fn test_admin_lock_account() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        let user_id = simulator.admin_create_user(
            admin_id,
            "lockme@example.com",
            "lockme",
            "user"
        ).await.unwrap();

        // Lock account for 24 hours
        let result = simulator.admin_lock_account(admin_id, user_id, 24).await;
        assert!(result.is_ok(), "Admin should be able to lock account");

        let users = simulator.users.lock().unwrap();
        let user = users.get(&user_id).unwrap();
        assert!(matches!(user.status, UserStatus::Locked));
        assert!(user.locked_until.is_some());
    }

    // Test bulk operations
    #[tokio::test]
    async fn test_admin_bulk_operations() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // Create multiple users
        let mut user_ids = Vec::new();
        for i in 0..5 {
            let user_id = simulator.admin_create_user(
                admin_id,
                &format!("bulk{}@example.com", i),
                &format!("bulk{}", i),
                "user"
            ).await.unwrap();
            user_ids.push(user_id);
        }

        // Perform bulk role assignment
        let result = simulator.admin_bulk_operation(
            admin_id,
            BulkOperationType::AssignRole("moderator".to_string()),
            user_ids.clone()
        ).await;

        assert!(result.is_ok(), "Bulk operation should succeed");
        let operation_id = result.unwrap();

        // Verify bulk operation results
        let operations = simulator.bulk_operations.lock().unwrap();
        let operation = operations.iter().find(|op| op.id == operation_id).unwrap();
        assert_eq!(operation.success_count, 5);
        assert_eq!(operation.failure_count, 0);
        assert!(matches!(operation.status, BulkOperationStatus::Completed));

        // Verify all users got the role
        let users = simulator.users.lock().unwrap();
        for user_id in &user_ids {
            let user = users.get(user_id).unwrap();
            assert!(user.roles.contains("moderator"));
        }
    }

    // Test audit logging
    #[tokio::test]
    async fn test_admin_audit_logging() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // Perform various admin actions
        let user_id = simulator.admin_create_user(
            admin_id,
            "audit@example.com",
            "audit",
            "user"
        ).await.unwrap();

        simulator.admin_assign_role(admin_id, user_id, "moderator").await.unwrap();
        simulator.admin_lock_account(admin_id, user_id, 1).await.unwrap();

        // Get audit log
        let audit_log = simulator.get_audit_log(admin_id, 10).unwrap();

        assert!(audit_log.len() >= 3, "Should have at least 3 audit entries");

        // Verify audit entries
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::CreateUser)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::AssignRole)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::LockAccount)));

        // All entries should be successful
        assert!(audit_log.iter().all(|e| e.success));
    }

    // Test permission enforcement
    #[tokio::test]
    async fn test_admin_permission_enforcement() {
        let simulator = AdminManagementSimulator::new();

        // Create a non-admin user
        let non_admin_id = Uuid::now_v7();
        let non_admin = ManagedUser {
            id: non_admin_id,
            email: "notadmin@example.com".to_string(),
            username: "notadmin".to_string(),
            status: UserStatus::Active,
            roles: HashSet::from(["user".to_string()]),
            permissions: HashSet::from(["profile.read".to_string(), "profile.update".to_string()]),
            created_at: Utc::now(),
            created_by: Uuid::nil(),
            last_modified: Utc::now(),
            modified_by: None,
            locked_until: None,
            failed_login_attempts: 0,
            email_verified: true,
            two_factor_enabled: false,
            metadata: HashMap::new(),
        };

        {
            let mut users = simulator.users.lock().unwrap();
            users.insert(non_admin_id, non_admin);
        }

        // Try to perform admin actions with non-admin user
        let result = simulator.admin_create_user(
            non_admin_id,
            "test@example.com",
            "test",
            "user"
        ).await;

        assert!(result.is_err(), "Non-admin should not be able to create users");
        assert_eq!(result.unwrap_err(), "Permission denied");

        // Verify audit log shows failed attempt
        let (admin_id, _) = simulator.create_admin_user();
        let audit_log = simulator.get_audit_log(admin_id, 10).unwrap();

        assert!(audit_log.iter().any(|e|
            e.admin_id == non_admin_id &&
            !e.success &&
            e.error_message.as_ref().map_or(false, |msg| msg.contains("Permission denied"))
        ));
    }

    // Test system settings update
    #[tokio::test]
    async fn test_admin_system_settings() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        let new_settings = SystemSettings {
            max_login_attempts: 3,
            lockout_duration_minutes: 60,
            password_expiry_days: 60,
            require_two_factor: true,
            audit_retention_days: 180,
            allow_self_registration: false,
            default_user_role: "restricted".to_string(),
        };

        let result = simulator.update_system_settings(admin_id, new_settings.clone()).await;
        assert!(result.is_ok(), "Admin should be able to update system settings");

        let settings = simulator.system_settings.lock().unwrap();
        assert_eq!(settings.max_login_attempts, 3);
        assert_eq!(settings.lockout_duration_minutes, 60);
        assert!(settings.require_two_factor);
        assert!(!settings.allow_self_registration);
    }

    // Test user statistics
    #[tokio::test]
    async fn test_admin_user_statistics() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // Create various users with different statuses
        for i in 0..10 {
            let user_id = simulator.admin_create_user(
                admin_id,
                &format!("stats{}@example.com", i),
                &format!("stats{}", i),
                "user"
            ).await.unwrap();

            if i < 3 {
                simulator.admin_lock_account(admin_id, user_id, 1).await.unwrap();
            }
        }

        let stats = simulator.get_user_statistics(admin_id).unwrap();

        assert!(stats.get("total_users").unwrap() >= &11); // 10 created + 1 admin
        assert!(stats.get("locked_users").unwrap() >= &3);
        assert!(stats.get("admin_count").unwrap() >= &1);
        assert!(stats.get("pending_verification").unwrap() >= &7); // Users created in pending state
    }

    // Test preventing deletion of last admin
    #[tokio::test]
    async fn test_prevent_last_admin_deletion() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // Try to remove admin role from self (last admin)
        let result = simulator.admin_remove_role(admin_id, admin_id, "admin").await;

        assert!(result.is_err(), "Should not be able to remove last admin role");
        assert!(result.unwrap_err().contains("Cannot remove last admin role"));
    }

    // Test comprehensive admin workflow
    #[tokio::test]
    async fn test_comprehensive_admin_workflow() {
        let simulator = AdminManagementSimulator::new();
        let (admin_id, _session) = simulator.create_admin_user();

        // 1. Create multiple users
        let mut user_ids = Vec::new();
        for i in 0..3 {
            let user_id = simulator.admin_create_user(
                admin_id,
                &format!("workflow{}@example.com", i),
                &format!("workflow{}", i),
                "user"
            ).await.unwrap();
            user_ids.push(user_id);
        }

        // 2. Bulk assign moderator role
        simulator.admin_bulk_operation(
            admin_id,
            BulkOperationType::AssignRole("moderator".to_string()),
            user_ids.clone()
        ).await.unwrap();

        // 3. Lock one account
        simulator.admin_lock_account(admin_id, user_ids[0], 24).await.unwrap();

        // 4. Delete another account
        simulator.admin_delete_user(admin_id, user_ids[1]).await.unwrap();

        // 5. Update system settings
        let new_settings = SystemSettings {
            max_login_attempts: 3,
            lockout_duration_minutes: 60,
            password_expiry_days: 30,
            require_two_factor: true,
            audit_retention_days: 90,
            allow_self_registration: false,
            default_user_role: "restricted".to_string(),
        };
        simulator.update_system_settings(admin_id, new_settings).await.unwrap();

        // 6. Verify statistics
        let stats = simulator.get_user_statistics(admin_id).unwrap();
        assert!(stats.get("locked_users").unwrap() >= &1);

        // 7. Verify audit log
        let audit_log = simulator.get_audit_log(admin_id, 20).unwrap();
        assert!(audit_log.len() >= 5); // At least 5 actions performed

        // Verify all expected actions are in audit log
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::CreateUser)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::BulkOperation)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::LockAccount)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::DeleteUser)));
        assert!(audit_log.iter().any(|e| matches!(e.action, AdminAction::UpdateSystemSettings)));

        println!("✅ Comprehensive admin workflow completed successfully!");
        println!("📊 Total audit entries: {}", audit_log.len());
        println!("📈 User statistics: {:?}", stats);
    }
}