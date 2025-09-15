//! Integration Tests for Account Deactivation and Reactivation Flows
//!
//! This module contains integration tests for the account lifecycle management.
//! These tests simulate the complete deactivation and reactivation flows without requiring a live database.

#[cfg(test)]
mod account_deactivation_tests {
    use chrono::{Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;
    use serde::Serialize;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::sync::{Arc, Mutex};

    use crate::domain::errors::AppError;
    use crate::domain::models::{User, UserStatus};

    #[derive(Debug, Clone)]
    struct DeactivationRecord {
        id: Uuid,
        user_id: Uuid,
        reason: Option<String>,
        deactivated_by: Uuid, // Self-deactivated or admin
        deactivated_at: chrono::DateTime<Utc>,
        scheduled_deletion_at: Option<chrono::DateTime<Utc>>,
        is_reversible: bool,
    }

    #[derive(Debug, Clone)]
    struct ReactivationToken {
        token: String,
        user_id: Uuid,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        used: bool,
        email_sent: bool,
    }

    #[derive(Debug, Clone)]
    struct SessionInfo {
        id: String,
        user_id: Uuid,
        token: String,
        created_at: chrono::DateTime<Utc>,
        expires_at: chrono::DateTime<Utc>,
        ip_address: String,
        is_active: bool,
    }

    #[derive(Debug, Clone, Serialize)]
    struct AccountActivity {
        user_id: Uuid,
        activity_type: String,
        details: String,
        timestamp: chrono::DateTime<Utc>,
        ip_address: String,
    }

    #[derive(Debug, Clone, Serialize)]
    struct DataRetentionPolicy {
        user_id: Uuid,
        retention_period_days: u32,
        data_types: HashSet<String>,
        deletion_scheduled_at: chrono::DateTime<Utc>,
        compliance_reason: String,
        approved_by: Option<Uuid>,
    }

    #[derive(Debug, Clone, Serialize)]
    struct DeletionAuditLog {
        id: Uuid,
        user_id: Uuid,
        deleted_at: chrono::DateTime<Utc>,
        deleted_by: Uuid,
        deletion_type: DeletionType,
        data_removed: HashMap<String, usize>, // data type -> count
        compliance_notes: Option<String>,
        recovery_possible: bool,
        backup_location: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, PartialEq)]
    enum DeletionType {
        SoftDelete,
        HardDelete,
        GdprErasure,
        ScheduledCleanup,
        AdminAction,
    }

    #[derive(Debug, Clone, Serialize)]
    struct DeactivationNotification {
        id: Uuid,
        user_id: Uuid,
        notification_type: NotificationType,
        sent_at: chrono::DateTime<Utc>,
        delivery_method: String,
        status: DeliveryStatus,
        retry_count: u32,
        content: String,
    }

    #[derive(Debug, Clone, Serialize, PartialEq)]
    enum NotificationType {
        DeactivationConfirmation,
        DeletionWarning,
        ReactivationAvailable,
        DataExportReady,
        FinalWarning,
    }

    #[derive(Debug, Clone, Serialize, PartialEq)]
    enum DeliveryStatus {
        Pending,
        Sent,
        Delivered,
        Failed,
        Bounced,
    }

    #[derive(Debug, Clone, Serialize)]
    struct UserDataExport {
        export_id: Uuid,
        user_id: Uuid,
        requested_at: chrono::DateTime<Utc>,
        completed_at: Option<chrono::DateTime<Utc>>,
        data_types: HashSet<String>,
        export_format: String,
        file_size_bytes: Option<u64>,
        download_url: Option<String>,
        expires_at: chrono::DateTime<Utc>,
        downloaded_count: u32,
    }

    #[derive(Debug, Clone, Serialize)]
    struct RecoveryAttempt {
        attempt_id: Uuid,
        user_id: Uuid,
        attempted_at: chrono::DateTime<Utc>,
        method: RecoveryMethod,
        success: bool,
        ip_address: String,
        user_agent: String,
        failure_reason: Option<String>,
        security_flags: HashSet<String>,
    }

    #[derive(Debug, Clone, Serialize, PartialEq)]
    enum RecoveryMethod {
        EmailToken,
        SmsCode,
        AdminOverride,
        SupportTicket,
        IdentityVerification,
    }

    // Mock account deactivation simulator with enhanced features
    struct AccountDeactivationSimulator {
        users: Vec<User>,
        deactivation_records: Vec<DeactivationRecord>,
        reactivation_tokens: Vec<ReactivationToken>,
        active_sessions: Vec<SessionInfo>,
        activity_logs: Vec<AccountActivity>,
        deleted_accounts: Vec<Uuid>, // Track permanently deleted accounts

        // Enhanced features
        data_retention_policies: Vec<DataRetentionPolicy>,
        deletion_audit_logs: Vec<DeletionAuditLog>,
        deactivation_notifications: Vec<DeactivationNotification>,
        data_exports: Vec<UserDataExport>,
        recovery_attempts: Vec<RecoveryAttempt>,
        rate_limits: HashMap<String, VecDeque<chrono::DateTime<Utc>>>, // IP -> timestamps
        fraud_detection: HashMap<Uuid, HashSet<String>>, // user_id -> suspicious patterns
        compliance_flags: HashMap<Uuid, HashSet<String>>, // user_id -> compliance requirements
        backup_metadata: HashMap<Uuid, String>, // user_id -> backup reference
        notification_preferences: HashMap<Uuid, HashSet<String>>, // user_id -> channels
    }

    impl AccountDeactivationSimulator {
        fn new() -> Self {
            Self {
                users: Vec::new(),
                deactivation_records: Vec::new(),
                reactivation_tokens: Vec::new(),
                active_sessions: Vec::new(),
                activity_logs: Vec::new(),
                deleted_accounts: Vec::new(),

                // Enhanced features
                data_retention_policies: Vec::new(),
                deletion_audit_logs: Vec::new(),
                deactivation_notifications: Vec::new(),
                data_exports: Vec::new(),
                recovery_attempts: Vec::new(),
                rate_limits: HashMap::new(),
                fraud_detection: HashMap::new(),
                compliance_flags: HashMap::new(),
                backup_metadata: HashMap::new(),
                notification_preferences: HashMap::new(),
            }
        }

        fn create_test_user(&mut self, email: String, username: String) -> User {
            let user = User {
                id: Uuid::new_v4(),
                username,
                email: email.clone(),
                password_hash: "hashed_password".to_string(),
                email_verified: true,
                status: UserStatus::Active,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login_at: Some(Utc::now()),
            };

            self.users.push(user.clone());
            user
        }

        fn create_session(&mut self, user_id: &Uuid, ip_address: String) -> SessionInfo {
            let session = SessionInfo {
                id: Uuid::new_v4().to_string(),
                user_id: *user_id,
                token: format!("session_{}", Uuid::new_v4()),
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(24),
                ip_address,
                is_active: true,
            };

            self.active_sessions.push(session.clone());
            session
        }

        fn deactivate_account(&mut self, user_id: &Uuid, deactivated_by: &Uuid, reason: Option<String>, ip_address: String) -> Result<(), AppError> {
            // Find and validate user
            let user_index = self.users.iter()
                .position(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            let user = &self.users[user_index];

            // Check if user is already deactivated
            if user.status == UserStatus::Inactive {
                return Err(AppError::Validation {
                    field: "user_status".to_string(),
                    message: "Account is already deactivated".to_string(),
                });
            }

            // Check if user is suspended (different from deactivated)
            if user.status == UserStatus::Suspended {
                return Err(AppError::Validation {
                    field: "user_status".to_string(),
                    message: "Cannot deactivate suspended account directly".to_string(),
                });
            }

            let is_self_deactivation = user_id == deactivated_by;
            let now = Utc::now();

            // Create deactivation record
            let deactivation_record = DeactivationRecord {
                id: Uuid::new_v4(),
                user_id: *user_id,
                reason: reason.clone(),
                deactivated_by: *deactivated_by,
                deactivated_at: now,
                scheduled_deletion_at: if is_self_deactivation {
                    Some(now + Duration::days(30)) // 30-day grace period for self-deactivation
                } else {
                    None // Admin deactivation doesn't auto-delete
                },
                is_reversible: true,
            };

            self.deactivation_records.push(deactivation_record);

            // Update user status
            self.users[user_index].status = UserStatus::Inactive;
            self.users[user_index].updated_at = now;

            // Revoke all active sessions
            for session in self.active_sessions.iter_mut() {
                if session.user_id == *user_id {
                    session.is_active = false;
                }
            }

            // Log activity
            let activity = AccountActivity {
                user_id: *user_id,
                activity_type: "account_deactivated".to_string(),
                details: format!(
                    "Account deactivated by {}. Reason: {:?}. Self-deactivation: {}",
                    if is_self_deactivation { "user" } else { "admin" },
                    reason.unwrap_or_else(|| "No reason provided".to_string()),
                    is_self_deactivation
                ),
                timestamp: now,
                ip_address,
            };
            self.activity_logs.push(activity);

            // Create deactivation notifications
            if let Some(channels) = self.notification_preferences.get(user_id) {
                for channel in channels {
                    let notification = DeactivationNotification {
                        id: Uuid::new_v4(),
                        user_id: *user_id,
                        notification_type: NotificationType::DeactivationConfirmation,
                        sent_at: now,
                        delivery_method: channel.clone(),
                        status: DeliveryStatus::Sent,
                        retry_count: 0,
                        content: format!(
                            "Your account has been deactivated. Reason: {}",
                            reason.clone().unwrap_or_else(|| "No reason provided".to_string())
                        ),
                    };
                    self.deactivation_notifications.push(notification);
                }
            }

            Ok(())
        }

        fn request_reactivation(&mut self, user_id: &Uuid, ip_address: String) -> Result<String, AppError> {
            // Find user
            let user = self.users.iter()
                .find(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Check if user is deactivated
            if user.status != UserStatus::Inactive {
                return Err(AppError::Validation {
                    field: "user_status".to_string(),
                    message: "Account is not deactivated".to_string(),
                });
            }

            // Check if account is scheduled for deletion
            if let Some(deactivation) = self.deactivation_records.iter().find(|d| d.user_id == *user_id) {
                if let Some(deletion_time) = deactivation.scheduled_deletion_at {
                    if Utc::now() > deletion_time {
                        return Err(AppError::Validation {
                            field: "account_status".to_string(),
                            message: "Account is scheduled for deletion and cannot be reactivated".to_string(),
                        });
                    }
                }

                if !deactivation.is_reversible {
                    return Err(AppError::Validation {
                        field: "account_status".to_string(),
                        message: "Account deactivation is not reversible".to_string(),
                    });
                }
            }

            // Invalidate any existing reactivation tokens
            for token in self.reactivation_tokens.iter_mut() {
                if token.user_id == *user_id && !token.used {
                    token.used = true;
                }
            }

            // Create new reactivation token
            let token = format!("reactivate_{}", Uuid::new_v4());
            let reactivation_token = ReactivationToken {
                token: token.clone(),
                user_id: *user_id,
                created_at: Utc::now(),
                expires_at: Utc::now() + Duration::hours(24), // 24-hour expiration
                used: false,
                email_sent: true,
            };

            self.reactivation_tokens.push(reactivation_token);

            // Log activity
            let activity = AccountActivity {
                user_id: *user_id,
                activity_type: "reactivation_requested".to_string(),
                details: "Account reactivation requested".to_string(),
                timestamp: Utc::now(),
                ip_address,
            };
            self.activity_logs.push(activity);

            Ok(token)
        }

        fn reactivate_account(&mut self, token: &str, ip_address: String) -> Result<User, AppError> {
            // Find and validate token
            let token_index = self.reactivation_tokens.iter()
                .position(|t| t.token == token && !t.used)
                .ok_or_else(|| AppError::Validation {
                    field: "token".to_string(),
                    message: "Invalid or expired reactivation token".to_string(),
                })?;

            let reactivation_token = &self.reactivation_tokens[token_index];

            // Check token expiration
            if reactivation_token.expires_at < Utc::now() {
                return Err(AppError::Validation {
                    field: "token".to_string(),
                    message: "Reactivation token has expired".to_string(),
                });
            }

            let user_id = reactivation_token.user_id;

            // Find and update user
            let user_index = self.users.iter()
                .position(|u| u.id == user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Update user status
            self.users[user_index].status = UserStatus::Active;
            self.users[user_index].updated_at = Utc::now();

            // Mark token as used
            self.reactivation_tokens[token_index].used = true;

            // Remove deactivation record (or mark as resolved)
            if let Some(deactivation_index) = self.deactivation_records.iter().position(|d| d.user_id == user_id) {
                self.deactivation_records.remove(deactivation_index);
            }

            // Log activity
            let activity = AccountActivity {
                user_id,
                activity_type: "account_reactivated".to_string(),
                details: "Account successfully reactivated".to_string(),
                timestamp: Utc::now(),
                ip_address,
            };
            self.activity_logs.push(activity);

            Ok(self.users[user_index].clone())
        }

        fn permanently_delete_account(&mut self, user_id: &Uuid, admin_user_id: &Uuid, ip_address: String) -> Result<(), AppError> {
            // Find deactivation record
            let deactivation = self.deactivation_records.iter()
                .find(|d| d.user_id == *user_id)
                .ok_or_else(|| AppError::Validation {
                    field: "account".to_string(),
                    message: "Account must be deactivated before permanent deletion".to_string(),
                })?;

            // Check if deletion is allowed
            // Admins can always delete, users must wait for scheduled time
            let is_admin_deletion = admin_user_id != user_id;
            let can_delete = if is_admin_deletion {
                true // Admin can delete immediately regardless of schedule
            } else if let Some(scheduled_time) = deactivation.scheduled_deletion_at {
                Utc::now() >= scheduled_time // User must wait for scheduled time
            } else {
                false // No scheduled deletion and not admin
            };

            if !can_delete {
                return Err(AppError::Validation {
                    field: "deletion_time".to_string(),
                    message: "Account is not yet eligible for deletion".to_string(),
                });
            }

            // Remove user from users list
            if let Some(user_index) = self.users.iter().position(|u| u.id == *user_id) {
                self.users.remove(user_index);
            }

            // Clean up related data
            self.deactivation_records.retain(|d| d.user_id != *user_id);
            self.reactivation_tokens.retain(|t| t.user_id != *user_id);
            self.active_sessions.retain(|s| s.user_id != *user_id);

            // Track deletion
            self.deleted_accounts.push(*user_id);

            // Log activity
            let activity = AccountActivity {
                user_id: *user_id,
                activity_type: "account_permanently_deleted".to_string(),
                details: format!("Account permanently deleted by admin {}", admin_user_id),
                timestamp: Utc::now(),
                ip_address,
            };
            self.activity_logs.push(activity);

            Ok(())
        }

        fn get_deactivation_info(&self, user_id: &Uuid) -> Option<&DeactivationRecord> {
            self.deactivation_records.iter().find(|d| d.user_id == *user_id)
        }

        fn get_account_status(&self, user_id: &Uuid) -> Result<(UserStatus, Option<DeactivationRecord>), AppError> {
            if self.deleted_accounts.contains(user_id) {
                return Err(AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                });
            }

            let user = self.users.iter()
                .find(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            let deactivation_info = self.get_deactivation_info(user_id).cloned();
            Ok((user.status.clone(), deactivation_info))
        }

        fn cleanup_expired_tokens(&mut self) {
            let now = Utc::now();
            for token in self.reactivation_tokens.iter_mut() {
                if token.expires_at < now {
                    token.used = true;
                }
            }
        }

        fn process_scheduled_deletions(&mut self, admin_user_id: &Uuid) -> Vec<Uuid> {
            let now = Utc::now();
            let mut deleted_user_ids = Vec::new();

            let users_to_delete: Vec<Uuid> = self.deactivation_records.iter()
                .filter(|d| {
                    if let Some(deletion_time) = d.scheduled_deletion_at {
                        deletion_time <= now
                    } else {
                        false
                    }
                })
                .map(|d| d.user_id)
                .collect();

            for user_id in users_to_delete {
                if self.permanently_delete_account(&user_id, admin_user_id, "system".to_string()).is_ok() {
                    deleted_user_ids.push(user_id);
                }
            }

            deleted_user_ids
        }

        fn get_user_activity_logs(&self, user_id: &Uuid) -> Vec<&AccountActivity> {
            self.activity_logs.iter()
                .filter(|log| log.user_id == *user_id)
                .collect()
        }

        fn revoke_all_sessions(&mut self, user_id: &Uuid) -> usize {
            let mut revoked_count = 0;
            for session in self.active_sessions.iter_mut() {
                if session.user_id == *user_id && session.is_active {
                    session.is_active = false;
                    revoked_count += 1;
                }
            }
            revoked_count
        }

        fn get_active_sessions(&self, user_id: &Uuid) -> Vec<&SessionInfo> {
            self.active_sessions.iter()
                .filter(|s| s.user_id == *user_id && s.is_active)
                .collect()
        }

        // Enhanced Methods for Advanced Testing

        fn set_compliance_flag(&mut self, user_id: &Uuid, flag: String) {
            self.compliance_flags.entry(*user_id)
                .or_insert_with(HashSet::new)
                .insert(flag);
        }

        fn gdpr_delete_account(&mut self, user_id: &Uuid, deleted_by: &Uuid, ip_address: String) -> Result<(), AppError> {
            // Check if user exists
            let user_index = self.users.iter()
                .position(|u| u.id == *user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "user".to_string(),
                    id: Some(user_id.to_string()),
                })?;

            // Create deletion audit log
            let audit_log = DeletionAuditLog {
                id: Uuid::new_v4(),
                user_id: *user_id,
                deleted_at: Utc::now(),
                deleted_by: *deleted_by,
                deletion_type: DeletionType::GdprErasure,
                data_removed: HashMap::from([
                    ("profile".to_string(), 1),
                    ("sessions".to_string(), self.active_sessions.iter().filter(|s| s.user_id == *user_id).count()),
                    ("activity_logs".to_string(), self.activity_logs.iter().filter(|a| a.user_id == *user_id).count()),
                ]),
                compliance_notes: Some("GDPR right to erasure".to_string()),
                recovery_possible: false,
                backup_location: None,
            };
            self.deletion_audit_logs.push(audit_log);

            // Remove all user data
            self.users.remove(user_index);
            self.deleted_accounts.push(*user_id);
            self.deactivation_records.retain(|d| d.user_id != *user_id);
            self.reactivation_tokens.retain(|t| t.user_id != *user_id);
            self.active_sessions.retain(|s| s.user_id != *user_id);
            self.activity_logs.retain(|a| a.user_id != *user_id);
            self.data_retention_policies.retain(|p| p.user_id != *user_id);
            self.deactivation_notifications.retain(|n| n.user_id != *user_id);
            self.data_exports.retain(|e| e.user_id != *user_id);
            self.recovery_attempts.retain(|r| r.user_id != *user_id);

            Ok(())
        }

        fn get_deletion_audit_logs(&self, user_id: &Uuid) -> Vec<&DeletionAuditLog> {
            self.deletion_audit_logs.iter()
                .filter(|log| log.user_id == *user_id)
                .collect()
        }

        fn request_data_export(&mut self, user_id: &Uuid, data_types: Vec<String>) -> Result<Uuid, AppError> {
            let export_id = Uuid::new_v4();
            let export = UserDataExport {
                export_id,
                user_id: *user_id,
                requested_at: Utc::now(),
                completed_at: None,
                data_types: data_types.into_iter().collect(),
                export_format: "JSON".to_string(),
                file_size_bytes: None,
                download_url: None,
                expires_at: Utc::now() + Duration::days(7),
                downloaded_count: 0,
            };
            self.data_exports.push(export);
            Ok(export_id)
        }

        fn complete_data_export(&mut self, export_id: &Uuid, file_size: u64, download_url: String) -> Result<(), AppError> {
            let export = self.data_exports.iter_mut()
                .find(|e| e.export_id == *export_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "data_export".to_string(),
                    id: Some(export_id.to_string()),
                })?;

            export.completed_at = Some(Utc::now());
            export.file_size_bytes = Some(file_size);
            export.download_url = Some(download_url);
            Ok(())
        }

        fn get_data_export_status(&self, export_id: &Uuid) -> Result<&UserDataExport, AppError> {
            self.data_exports.iter()
                .find(|e| e.export_id == *export_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "data_export".to_string(),
                    id: Some(export_id.to_string()),
                })
        }

        fn create_retention_policy(&mut self, user_id: &Uuid, retention_days: u32, data_types: Vec<String>, reason: String) -> Result<&DataRetentionPolicy, AppError> {
            let policy = DataRetentionPolicy {
                user_id: *user_id,
                retention_period_days: retention_days,
                data_types: data_types.into_iter().collect(),
                deletion_scheduled_at: Utc::now() + Duration::days(retention_days as i64),
                compliance_reason: reason,
                approved_by: None,
            };
            self.data_retention_policies.push(policy);
            Ok(self.data_retention_policies.last().unwrap())
        }

        fn expire_retention_policy(&mut self, user_id: &Uuid) {
            for policy in self.data_retention_policies.iter_mut() {
                if policy.user_id == *user_id {
                    policy.deletion_scheduled_at = Utc::now() - Duration::days(1);
                }
            }
        }

        fn attempt_account_recovery(&mut self, user_id: &Uuid, method: RecoveryMethod, ip_address: String, user_agent: String) -> Result<RecoveryAttempt, AppError> {
            // Check rate limiting
            let now = Utc::now();
            let rate_limit_key = ip_address.clone();
            let attempts = self.rate_limits.entry(rate_limit_key).or_insert_with(VecDeque::new);

            // Remove old attempts (older than 1 hour)
            while let Some(&front) = attempts.front() {
                if now.signed_duration_since(front).num_hours() >= 1 {
                    attempts.pop_front();
                } else {
                    break;
                }
            }

            // Check if rate limited (more than 5 attempts per hour)
            if attempts.len() >= 5 {
                return Err(AppError::TooManyRequests {
                    message: "Too many recovery attempts".to_string(),
                    retry_after: Some(3600),
                });
            }

            attempts.push_back(now);

            // Detect fraud patterns
            self.detect_recovery_fraud(user_id, &ip_address);

            let success = match method {
                RecoveryMethod::AdminOverride => true,
                RecoveryMethod::IdentityVerification => {
                    // Check if user has high security compliance
                    self.compliance_flags.get(user_id)
                        .map_or(false, |flags| flags.contains("HIGH_SECURITY"))
                },
                _ => attempts.len() <= 3, // Allow first few attempts
            };

            let attempt = RecoveryAttempt {
                attempt_id: Uuid::new_v4(),
                user_id: *user_id,
                attempted_at: now,
                method,
                success,
                ip_address,
                user_agent,
                failure_reason: if !success { Some("Rate limited or fraud detected".to_string()) } else { None },
                security_flags: HashSet::new(),
            };

            self.recovery_attempts.push(attempt.clone());
            Ok(attempt)
        }

        fn detect_recovery_fraud(&mut self, user_id: &Uuid, ip_address: &str) {
            // Count unique IPs used for recovery
            let unique_ips: HashSet<_> = self.recovery_attempts.iter()
                .filter(|r| r.user_id == *user_id)
                .map(|r| &r.ip_address)
                .collect();

            let fraud_flags = self.fraud_detection.entry(*user_id).or_insert_with(HashSet::new);

            if unique_ips.len() >= 3 {
                fraud_flags.insert("multiple_ip_recovery".to_string());
            }

            // Check for rapid attempts
            let recent_attempts = self.recovery_attempts.iter()
                .filter(|r| r.user_id == *user_id &&
                        Utc::now().signed_duration_since(r.attempted_at).num_minutes() <= 10)
                .count();

            if recent_attempts >= 5 {
                fraud_flags.insert("rapid_attempts".to_string());
            }
        }

        fn get_fraud_flags(&self, user_id: &Uuid) -> &HashSet<String> {
            self.fraud_detection.get(user_id).unwrap_or(&HashSet::new())
        }

        fn set_notification_preferences(&mut self, user_id: &Uuid, channels: Vec<String>) {
            self.notification_preferences.insert(*user_id, channels.into_iter().collect());
        }

        fn get_notifications(&self, user_id: &Uuid) -> Vec<&DeactivationNotification> {
            self.deactivation_notifications.iter()
                .filter(|n| n.user_id == *user_id)
                .collect()
        }

        fn mark_notification_delivered(&mut self, notification_id: &Uuid) -> Result<(), AppError> {
            let notification = self.deactivation_notifications.iter_mut()
                .find(|n| n.id == *notification_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "notification".to_string(),
                    id: Some(notification_id.to_string()),
                })?;

            notification.status = DeliveryStatus::Delivered;
            Ok(())
        }

        fn cascade_delete_account(&mut self, user_id: &Uuid, deleted_by: &Uuid, ip_address: String) -> Result<(), AppError> {
            let mut data_removed = HashMap::new();

            // Count what will be deleted
            data_removed.insert("sessions".to_string(), self.active_sessions.iter().filter(|s| s.user_id == *user_id).count());
            data_removed.insert("exports".to_string(), self.data_exports.iter().filter(|e| e.user_id == *user_id).count());
            data_removed.insert("policies".to_string(), self.data_retention_policies.iter().filter(|p| p.user_id == *user_id).count());
            data_removed.insert("notifications".to_string(), self.deactivation_notifications.iter().filter(|n| n.user_id == *user_id).count());
            data_removed.insert("recovery_attempts".to_string(), self.recovery_attempts.iter().filter(|r| r.user_id == *user_id).count());

            // Create audit log
            let audit_log = DeletionAuditLog {
                id: Uuid::new_v4(),
                user_id: *user_id,
                deleted_at: Utc::now(),
                deleted_by: *deleted_by,
                deletion_type: DeletionType::AdminAction,
                data_removed,
                compliance_notes: Some("Cascading deletion with all dependencies".to_string()),
                recovery_possible: false,
                backup_location: self.backup_metadata.get(user_id).cloned(),
            };
            self.deletion_audit_logs.push(audit_log);

            // Remove all related data
            self.active_sessions.retain(|s| s.user_id != *user_id);
            self.data_exports.retain(|e| e.user_id != *user_id);
            self.data_retention_policies.retain(|p| p.user_id != *user_id);
            self.deactivation_notifications.retain(|n| n.user_id != *user_id);
            self.recovery_attempts.retain(|r| r.user_id != *user_id);
            self.fraud_detection.remove(user_id);
            self.compliance_flags.remove(user_id);
            self.notification_preferences.remove(user_id);

            // Finally delete the user
            self.permanently_delete_account(user_id, deleted_by, ip_address)
        }

        fn create_account_backup(&mut self, user_id: &Uuid, admin_id: &Uuid) -> Result<String, AppError> {
            let backup_ref = format!("backup_{}_{}", user_id, Utc::now().timestamp());
            self.backup_metadata.insert(*user_id, backup_ref.clone());

            // Log backup creation
            let activity = AccountActivity {
                user_id: *user_id,
                activity_type: "backup_created".to_string(),
                details: format!("Account backup created by admin {}", admin_id),
                timestamp: Utc::now(),
                ip_address: "internal".to_string(),
            };
            self.activity_logs.push(activity);

            Ok(backup_ref)
        }

        fn get_backup_metadata(&self, user_id: &Uuid) -> Result<&String, AppError> {
            self.backup_metadata.get(user_id)
                .ok_or_else(|| AppError::NotFound {
                    resource: "backup".to_string(),
                    id: Some(user_id.to_string()),
                })
        }

        fn verify_backup_integrity(&self, backup_ref: &str) -> Result<bool, AppError> {
            // Simulate backup verification
            Ok(backup_ref.starts_with("backup_"))
        }

        fn simulate_time_passage(&mut self, duration: Duration) {
            // In a real system, this would advance the system clock
            // For testing, we can adjust scheduled times
            for policy in self.data_retention_policies.iter_mut() {
                policy.deletion_scheduled_at = policy.deletion_scheduled_at - duration;
            }
        }

        fn process_deletion_timeline_notifications(&mut self) {
            let now = Utc::now();

            for record in &self.deactivation_records {
                if let Some(deletion_time) = record.scheduled_deletion_at {
                    let days_until_deletion = deletion_time.signed_duration_since(now).num_days();

                    let notification_type = if days_until_deletion <= 1 {
                        NotificationType::FinalWarning
                    } else if days_until_deletion <= 7 {
                        NotificationType::DeletionWarning
                    } else {
                        continue;
                    };

                    // Check if this notification type was already sent
                    let already_sent = self.deactivation_notifications.iter()
                        .any(|n| n.user_id == record.user_id && n.notification_type == notification_type);

                    if !already_sent {
                        if let Some(channels) = self.notification_preferences.get(&record.user_id) {
                            for channel in channels {
                                let notification = DeactivationNotification {
                                    id: Uuid::new_v4(),
                                    user_id: record.user_id,
                                    notification_type: notification_type.clone(),
                                    sent_at: now,
                                    delivery_method: channel.clone(),
                                    status: DeliveryStatus::Sent,
                                    retry_count: 0,
                                    content: format!("Your account will be deleted in {} days", days_until_deletion),
                                };
                                self.deactivation_notifications.push(notification);
                            }
                        }
                    }
                }
            }
        }

        fn get_recovery_attempts(&self, user_id: &Uuid) -> Vec<&RecoveryAttempt> {
            self.recovery_attempts.iter()
                .filter(|r| r.user_id == *user_id)
                .collect()
        }
    }

    #[test]
    fn test_self_deactivation_flow() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Create some active sessions
        simulator.create_session(&user.id, "192.168.1.100".to_string());
        simulator.create_session(&user.id, "192.168.1.101".to_string());

        // Verify user has active sessions
        let active_sessions = simulator.get_active_sessions(&user.id);
        assert_eq!(active_sessions.len(), 2);

        // Self-deactivate account
        let result = simulator.deactivate_account(
            &user.id,
            &user.id, // Self-deactivation
            Some("Taking a break".to_string()),
            "192.168.1.100".to_string()
        );
        assert!(result.is_ok());

        // Verify account status
        let (status, deactivation_info) = simulator.get_account_status(&user.id).unwrap();
        assert_eq!(status, UserStatus::Inactive);

        let deactivation = deactivation_info.unwrap();
        assert_eq!(deactivation.user_id, user.id);
        assert_eq!(deactivation.deactivated_by, user.id);
        assert_eq!(deactivation.reason, Some("Taking a break".to_string()));
        assert!(deactivation.is_reversible);
        assert!(deactivation.scheduled_deletion_at.is_some());

        // Verify all sessions were revoked
        let active_sessions_after = simulator.get_active_sessions(&user.id);
        assert_eq!(active_sessions_after.len(), 0);

        // Verify activity was logged
        let activity_logs = simulator.get_user_activity_logs(&user.id);
        let deactivation_log = activity_logs.iter().find(|log| log.activity_type == "account_deactivated");
        assert!(deactivation_log.is_some());
    }

    #[test]
    fn test_admin_deactivation_flow() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );
        let admin_id = Uuid::new_v4();

        // Admin deactivate account
        let result = simulator.deactivate_account(
            &user.id,
            &admin_id,
            Some("Policy violation".to_string()),
            "10.0.0.1".to_string()
        );
        assert!(result.is_ok());

        // Verify account status
        let (status, deactivation_info) = simulator.get_account_status(&user.id).unwrap();
        assert_eq!(status, UserStatus::Inactive);

        let deactivation = deactivation_info.unwrap();
        assert_eq!(deactivation.deactivated_by, admin_id);
        assert_eq!(deactivation.reason, Some("Policy violation".to_string()));
        assert!(deactivation.is_reversible);
        assert!(deactivation.scheduled_deletion_at.is_none()); // Admin deactivation doesn't auto-schedule deletion
    }

    #[test]
    fn test_reactivation_flow() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate account
        simulator.deactivate_account(
            &user.id,
            &user.id,
            Some("Test deactivation".to_string()),
            "192.168.1.100".to_string()
        ).unwrap();

        // Request reactivation
        let token = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();
        assert!(!token.is_empty());
        assert!(token.starts_with("reactivate_"));

        // Verify reactivation token was created
        let reactivation_token = simulator.reactivation_tokens.iter()
            .find(|t| t.token == token)
            .unwrap();
        assert_eq!(reactivation_token.user_id, user.id);
        assert!(!reactivation_token.used);
        assert!(reactivation_token.email_sent);

        // Reactivate account using token
        let reactivated_user = simulator.reactivate_account(&token, "192.168.1.100".to_string()).unwrap();
        assert_eq!(reactivated_user.status, UserStatus::Active);
        assert_eq!(reactivated_user.id, user.id);

        // Verify token was marked as used
        let used_token = simulator.reactivation_tokens.iter()
            .find(|t| t.token == token)
            .unwrap();
        assert!(used_token.used);

        // Verify deactivation record was removed
        let deactivation_info = simulator.get_deactivation_info(&user.id);
        assert!(deactivation_info.is_none());
    }

    #[test]
    fn test_deactivate_already_deactivated_account() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // First deactivation
        simulator.deactivate_account(
            &user.id,
            &user.id,
            Some("First deactivation".to_string()),
            "192.168.1.100".to_string()
        ).unwrap();

        // Try to deactivate again
        let result = simulator.deactivate_account(
            &user.id,
            &user.id,
            Some("Second deactivation".to_string()),
            "192.168.1.100".to_string()
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "user_status");
                assert!(message.contains("already deactivated"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_reactivation_with_invalid_token() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate account
        simulator.deactivate_account(
            &user.id,
            &user.id,
            Some("Test".to_string()),
            "192.168.1.100".to_string()
        ).unwrap();

        // Try to reactivate with invalid token
        let result = simulator.reactivate_account("invalid_token", "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("Invalid or expired"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_reactivation_token_expiration() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate and get reactivation token
        simulator.deactivate_account(&user.id, &user.id, None, "192.168.1.100".to_string()).unwrap();
        let token = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();

        // Manually expire the token
        if let Some(token_ref) = simulator.reactivation_tokens.iter_mut().find(|t| t.token == token) {
            token_ref.expires_at = Utc::now() - Duration::hours(1);
        }

        // Try to use expired token
        let result = simulator.reactivate_account(&token, "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "token");
                assert!(message.contains("expired"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_reactivation_request_for_active_account() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Try to request reactivation for active account
        let result = simulator.request_reactivation(&user.id, "192.168.1.100".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::Validation { field, message } => {
                assert_eq!(field, "user_status");
                assert!(message.contains("not deactivated"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_permanent_deletion_flow() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );
        let admin_id = Uuid::new_v4();

        // Deactivate account first
        simulator.deactivate_account(&user.id, &user.id, Some("Leaving platform".to_string()), "192.168.1.100".to_string()).unwrap();

        // Permanently delete account
        let result = simulator.permanently_delete_account(&user.id, &admin_id, "10.0.0.1".to_string());
        assert!(result.is_ok());

        // Verify account is deleted
        let status_result = simulator.get_account_status(&user.id);
        assert!(status_result.is_err());

        // Verify user is in deleted accounts list
        assert!(simulator.deleted_accounts.contains(&user.id));

        // Verify all related data was cleaned up
        assert!(!simulator.deactivation_records.iter().any(|d| d.user_id == user.id));
        assert!(!simulator.reactivation_tokens.iter().any(|t| t.user_id == user.id));
        assert!(!simulator.active_sessions.iter().any(|s| s.user_id == user.id));
    }

    #[test]
    fn test_scheduled_deletion_processing() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user1 = simulator.create_test_user("user1@example.com".to_string(), "user1".to_string());
        let user2 = simulator.create_test_user("user2@example.com".to_string(), "user2".to_string());
        let admin_id = Uuid::new_v4();

        // Deactivate accounts with scheduled deletion
        simulator.deactivate_account(&user1.id, &user1.id, Some("Test 1".to_string()), "192.168.1.100".to_string()).unwrap();
        simulator.deactivate_account(&user2.id, &user2.id, Some("Test 2".to_string()), "192.168.1.100".to_string()).unwrap();

        // Manually set deletion times to past (to simulate expired grace period)
        for deactivation in simulator.deactivation_records.iter_mut() {
            if let Some(ref mut deletion_time) = deactivation.scheduled_deletion_at {
                *deletion_time = Utc::now() - Duration::days(1);
            }
        }

        // Process scheduled deletions
        let deleted_users = simulator.process_scheduled_deletions(&admin_id);
        assert_eq!(deleted_users.len(), 2);
        assert!(deleted_users.contains(&user1.id));
        assert!(deleted_users.contains(&user2.id));

        // Verify accounts were deleted
        assert!(simulator.get_account_status(&user1.id).is_err());
        assert!(simulator.get_account_status(&user2.id).is_err());
    }

    #[test]
    fn test_token_cleanup_functionality() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate and get reactivation token
        simulator.deactivate_account(&user.id, &user.id, None, "192.168.1.100".to_string()).unwrap();
        let token = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();

        // Verify token is active
        let active_token = simulator.reactivation_tokens.iter().find(|t| t.token == token && !t.used);
        assert!(active_token.is_some());

        // Manually expire the token
        if let Some(token_ref) = simulator.reactivation_tokens.iter_mut().find(|t| t.token == token) {
            token_ref.expires_at = Utc::now() - Duration::hours(1);
        }

        // Run cleanup
        simulator.cleanup_expired_tokens();

        // Verify token was marked as used
        let cleaned_token = simulator.reactivation_tokens.iter().find(|t| t.token == token);
        assert!(cleaned_token.unwrap().used);
    }

    #[test]
    fn test_multiple_reactivation_requests() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, None, "192.168.1.100".to_string()).unwrap();

        // Request reactivation multiple times
        let token1 = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();
        let token2 = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();
        let token3 = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();

        // Verify tokens are different
        assert_ne!(token1, token2);
        assert_ne!(token2, token3);

        // Verify only the latest token is valid (previous ones should be marked as used)
        let token1_status = simulator.reactivation_tokens.iter().find(|t| t.token == token1).unwrap();
        let token2_status = simulator.reactivation_tokens.iter().find(|t| t.token == token2).unwrap();
        let token3_status = simulator.reactivation_tokens.iter().find(|t| t.token == token3).unwrap();

        assert!(token1_status.used);
        assert!(token2_status.used);
        assert!(!token3_status.used);

        // Only the latest token should work for reactivation
        let result = simulator.reactivate_account(&token3, "192.168.1.100".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_revocation_on_deactivation() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Create multiple sessions
        let session1 = simulator.create_session(&user.id, "192.168.1.100".to_string());
        let session2 = simulator.create_session(&user.id, "192.168.1.101".to_string());
        let session3 = simulator.create_session(&user.id, "10.0.0.1".to_string());

        // Verify all sessions are active
        let active_sessions_before = simulator.get_active_sessions(&user.id);
        assert_eq!(active_sessions_before.len(), 3);

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, None, "192.168.1.100".to_string()).unwrap();

        // Verify all sessions were revoked
        let active_sessions_after = simulator.get_active_sessions(&user.id);
        assert_eq!(active_sessions_after.len(), 0);

        // Verify sessions exist but are marked as inactive
        let all_user_sessions: Vec<_> = simulator.active_sessions.iter()
            .filter(|s| s.user_id == user.id)
            .collect();
        assert_eq!(all_user_sessions.len(), 3);
        assert!(all_user_sessions.iter().all(|s| !s.is_active));
    }

    #[test]
    fn test_activity_logging_throughout_lifecycle() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, Some("Taking a break".to_string()), "192.168.1.100".to_string()).unwrap();

        // Request reactivation
        simulator.request_reactivation(&user.id, "192.168.1.101".to_string()).unwrap();

        // Get token and reactivate
        let token = simulator.reactivation_tokens.iter()
            .find(|t| t.user_id == user.id && !t.used)
            .unwrap()
            .token.clone();

        simulator.reactivate_account(&token, "192.168.1.102".to_string()).unwrap();

        // Verify all activities were logged
        let activity_logs = simulator.get_user_activity_logs(&user.id);
        assert_eq!(activity_logs.len(), 3);

        let deactivation_log = activity_logs.iter().find(|log| log.activity_type == "account_deactivated");
        let reactivation_request_log = activity_logs.iter().find(|log| log.activity_type == "reactivation_requested");
        let reactivation_log = activity_logs.iter().find(|log| log.activity_type == "account_reactivated");

        assert!(deactivation_log.is_some());
        assert!(reactivation_request_log.is_some());
        assert!(reactivation_log.is_some());

        // Verify IP addresses were tracked
        assert_eq!(deactivation_log.unwrap().ip_address, "192.168.1.100");
        assert_eq!(reactivation_request_log.unwrap().ip_address, "192.168.1.101");
        assert_eq!(reactivation_log.unwrap().ip_address, "192.168.1.102");
    }

    #[test]
    fn test_complete_deactivation_reactivation_cycle() {
        let mut simulator = AccountDeactivationSimulator::new();

        let user = simulator.create_test_user(
            "user@example.com".to_string(),
            "testuser".to_string()
        );

        // Initial state - active user
        let (initial_status, initial_deactivation) = simulator.get_account_status(&user.id).unwrap();
        assert_eq!(initial_status, UserStatus::Active);
        assert!(initial_deactivation.is_none());

        // Step 1: Deactivate account
        simulator.deactivate_account(
            &user.id,
            &user.id,
            Some("Need a break from the platform".to_string()),
            "192.168.1.100".to_string()
        ).unwrap();

        let (deactivated_status, deactivation_info) = simulator.get_account_status(&user.id).unwrap();
        assert_eq!(deactivated_status, UserStatus::Inactive);
        assert!(deactivation_info.is_some());

        // Step 2: Request reactivation
        let reactivation_token = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();
        assert!(!reactivation_token.is_empty());

        // Step 3: Complete reactivation
        let reactivated_user = simulator.reactivate_account(&reactivation_token, "192.168.1.100".to_string()).unwrap();
        assert_eq!(reactivated_user.status, UserStatus::Active);

        // Final state - active user again
        let (final_status, final_deactivation) = simulator.get_account_status(&user.id).unwrap();
        assert_eq!(final_status, UserStatus::Active);
        assert!(final_deactivation.is_none());

        // Verify complete activity trail
        let activity_logs = simulator.get_user_activity_logs(&user.id);
        assert_eq!(activity_logs.len(), 3);

        let activity_types: Vec<_> = activity_logs.iter().map(|log| &log.activity_type).collect();
        assert!(activity_types.contains(&&"account_deactivated".to_string()));
        assert!(activity_types.contains(&&"reactivation_requested".to_string()));
        assert!(activity_types.contains(&&"account_reactivated".to_string()));
    }

    // Enhanced Integration Tests with Advanced Features

    #[test]
    fn test_gdpr_compliant_data_deletion() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("gdpr@example.com".to_string(), "gdpruser".to_string());
        let admin_id = Uuid::new_v4();

        // Set GDPR compliance flag
        simulator.set_compliance_flag(&user.id, "GDPR".to_string());

        // Request GDPR erasure
        let result = simulator.gdpr_delete_account(&user.id, &admin_id, "192.168.1.100".to_string());
        assert!(result.is_ok());

        // Verify complete deletion
        assert!(simulator.get_account_status(&user.id).is_err());
        assert!(simulator.deleted_accounts.contains(&user.id));

        // Verify audit log created
        let audit_logs = simulator.get_deletion_audit_logs(&user.id);
        assert!(!audit_logs.is_empty());
        assert_eq!(audit_logs[0].deletion_type, DeletionType::GdprErasure);
        assert!(!audit_logs[0].recovery_possible);
    }

    #[test]
    fn test_data_export_before_deletion() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("export@example.com".to_string(), "exportuser".to_string());

        // Request data export
        let export_id = simulator.request_data_export(&user.id, vec!["profile".to_string(), "activity".to_string()]).unwrap();

        // Complete export processing
        simulator.complete_data_export(&export_id, 1024 * 1024, "https://downloads.example.com/export123".to_string()).unwrap();

        // Verify export is available
        let export_info = simulator.get_data_export_status(&export_id).unwrap();
        assert!(export_info.completed_at.is_some());
        assert_eq!(export_info.file_size_bytes, Some(1024 * 1024));
        assert!(export_info.download_url.is_some());

        // Now deactivate account
        simulator.deactivate_account(&user.id, &user.id, Some("Data exported".to_string()), "192.168.1.100".to_string()).unwrap();

        // Verify export is still accessible after deactivation
        let export_after = simulator.get_data_export_status(&export_id).unwrap();
        assert!(export_after.download_url.is_some());
    }

    #[test]
    fn test_retention_policy_enforcement() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("retention@example.com".to_string(), "retentionuser".to_string());
        let admin_id = Uuid::new_v4();

        // Create retention policy
        let _policy = simulator.create_retention_policy(&user.id, 7, vec!["activity_logs".to_string(), "sessions".to_string()], "Legal hold".to_string()).unwrap();

        // Deactivate account
        simulator.deactivate_account(&user.id, &admin_id, Some("Policy test".to_string()), "192.168.1.100".to_string()).unwrap();

        // Try to delete before retention period expires
        let early_delete = simulator.permanently_delete_account(&user.id, &admin_id, "192.168.1.100".to_string());
        assert!(early_delete.is_err());

        // Simulate retention period expiry
        simulator.expire_retention_policy(&user.id);

        // Now deletion should succeed
        let delete_result = simulator.permanently_delete_account(&user.id, &admin_id, "192.168.1.100".to_string());
        assert!(delete_result.is_ok());
    }

    #[test]
    fn test_fraud_detection_during_recovery() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("fraud@example.com".to_string(), "frauduser".to_string());

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, Some("Testing".to_string()), "192.168.1.100".to_string()).unwrap();

        // Multiple failed recovery attempts from different IPs
        let suspicious_ips = vec!["1.1.1.1", "2.2.2.2", "3.3.3.3", "4.4.4.4"];

        for ip in &suspicious_ips {
            let attempt = simulator.attempt_account_recovery(&user.id, RecoveryMethod::EmailToken, ip.to_string(), "SuspiciousAgent/1.0".to_string());
            // Should fail due to rate limiting/fraud detection
            assert!(attempt.is_err() || !attempt.unwrap().success);
        }

        // Verify fraud flags were set
        let fraud_flags = simulator.get_fraud_flags(&user.id);
        assert!(fraud_flags.contains("multiple_ip_recovery"));
        assert!(fraud_flags.contains("rapid_attempts"));

        // Legitimate recovery from original IP should still work
        let legitimate_recovery = simulator.attempt_account_recovery(&user.id, RecoveryMethod::AdminOverride, "192.168.1.100".to_string(), "LegitAgent/1.0".to_string());
        assert!(legitimate_recovery.is_ok());
    }

    #[test]
    fn test_notification_delivery_system() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("notify@example.com".to_string(), "notifyuser".to_string());

        // Set notification preferences
        simulator.set_notification_preferences(&user.id, vec!["email".to_string(), "sms".to_string()]);

        // Deactivate account (should trigger notifications)
        simulator.deactivate_account(&user.id, &user.id, Some("Testing notifications".to_string()), "192.168.1.100".to_string()).unwrap();

        // Check notifications were created
        let notifications = simulator.get_notifications(&user.id);
        assert!(!notifications.is_empty());

        let deactivation_notifications: Vec<_> = notifications.iter()
            .filter(|n| n.notification_type == NotificationType::DeactivationConfirmation)
            .collect();
        assert_eq!(deactivation_notifications.len(), 2); // email + sms

        // Simulate delivery - collect IDs first to avoid borrow issues
        let notification_ids: Vec<_> = notifications.iter().map(|n| n.id).collect();
        for notification_id in notification_ids {
            simulator.mark_notification_delivered(&notification_id).unwrap();
        }

        // Verify delivery status
        let updated_notifications = simulator.get_notifications(&user.id);
        for notification in &updated_notifications {
            assert_eq!(notification.status, DeliveryStatus::Delivered);
        }
    }

    #[test]
    fn test_cascading_deletion_with_dependencies() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("cascade@example.com".to_string(), "cascadeuser".to_string());
        let admin_id = Uuid::new_v4();

        // Create various related data
        simulator.create_session(&user.id, "192.168.1.100".to_string());
        let _export_id = simulator.request_data_export(&user.id, vec!["profile".to_string()]).unwrap();
        let _policy = simulator.create_retention_policy(&user.id, 1, vec!["logs".to_string()], "Test".to_string()).unwrap();

        // Verify dependencies exist
        assert!(!simulator.active_sessions.iter().filter(|s| s.user_id == user.id).collect::<Vec<_>>().is_empty());
        assert!(!simulator.data_exports.iter().filter(|e| e.user_id == user.id).collect::<Vec<_>>().is_empty());

        // Perform cascading deletion
        let deletion_result = simulator.cascade_delete_account(&user.id, &admin_id, "192.168.1.100".to_string());
        assert!(deletion_result.is_ok());

        // Verify all dependencies were cleaned up
        assert!(simulator.active_sessions.iter().filter(|s| s.user_id == user.id).collect::<Vec<_>>().is_empty());
        assert!(simulator.data_exports.iter().filter(|e| e.user_id == user.id).collect::<Vec<_>>().is_empty());
        assert!(simulator.data_retention_policies.iter().filter(|p| p.user_id == user.id).collect::<Vec<_>>().is_empty());

        // Verify audit log tracks what was deleted
        let audit_logs = simulator.get_deletion_audit_logs(&user.id);
        assert!(!audit_logs.is_empty());
        let log = &audit_logs[0];
        assert!(log.data_removed.contains_key("sessions"));
        assert!(log.data_removed.contains_key("exports"));
        assert!(log.data_removed.contains_key("policies"));
    }

    #[test]
    fn test_rate_limited_recovery_attempts() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("ratelimit@example.com".to_string(), "ratelimituser".to_string());
        let ip = "192.168.1.100";

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, Some("Rate limit test".to_string()), ip.to_string()).unwrap();

        // Attempt multiple recoveries rapidly
        let mut successful_attempts = 0;
        let mut rate_limited_attempts = 0;

        for i in 0..10 {
            let result = simulator.attempt_account_recovery(
                &user.id,
                RecoveryMethod::EmailToken,
                ip.to_string(),
                format!("TestAgent/{}", i)
            );

            match result {
                Ok(attempt) if attempt.success => successful_attempts += 1,
                Ok(_) => {} // Failed attempt but not rate limited
                Err(AppError::TooManyRequests { .. }) => rate_limited_attempts += 1,
                Err(_) => {} // Other error
            }
        }

        // Should have some rate limiting
        assert!(rate_limited_attempts > 0);
        assert!(successful_attempts <= 3); // First few attempts allowed

        // Different IP should not be rate limited initially
        let different_ip_result = simulator.attempt_account_recovery(
            &user.id,
            RecoveryMethod::EmailToken,
            "10.0.0.1".to_string(),
            "TestAgent/Different".to_string()
        );
        assert!(different_ip_result.is_ok());
    }

    #[test]
    fn test_backup_and_recovery_metadata() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("backup@example.com".to_string(), "backupuser".to_string());
        let admin_id = Uuid::new_v4();

        // Create backup before deletion
        let backup_ref = simulator.create_account_backup(&user.id, &admin_id).unwrap();
        assert!(!backup_ref.is_empty());

        // Verify backup metadata is stored
        let backup_info = simulator.get_backup_metadata(&user.id).unwrap();
        assert_eq!(backup_info, backup_ref);

        // Delete account
        simulator.permanently_delete_account(&user.id, &admin_id, "192.168.1.100".to_string()).unwrap();

        // Backup metadata should still exist even after deletion
        let backup_after_deletion = simulator.get_backup_metadata(&user.id);
        assert!(backup_after_deletion.is_ok());

        // Should be able to verify backup exists
        let backup_verification = simulator.verify_backup_integrity(&backup_ref);
        assert!(backup_verification.is_ok());
    }

    #[test]
    fn test_deletion_timeline_and_warnings() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("timeline@example.com".to_string(), "timelineuser".to_string());

        // Deactivate with self-deactivation (30-day deletion timeline)
        simulator.deactivate_account(&user.id, &user.id, Some("Timeline test".to_string()), "192.168.1.100".to_string()).unwrap();

        // Check initial notifications
        let initial_notifications = simulator.get_notifications(&user.id);
        let confirmation_notifications: Vec<_> = initial_notifications.iter()
            .filter(|n| n.notification_type == NotificationType::DeactivationConfirmation)
            .collect();
        assert!(!confirmation_notifications.is_empty());

        // Simulate time passing - 7 days before deletion
        simulator.simulate_time_passage(Duration::days(23)); // 30 - 23 = 7 days left

        // Process deletion warnings
        simulator.process_deletion_timeline_notifications();

        // Should have deletion warning
        let warning_notifications = simulator.get_notifications(&user.id);
        let deletion_warnings: Vec<_> = warning_notifications.iter()
            .filter(|n| n.notification_type == NotificationType::DeletionWarning)
            .collect();
        assert!(!deletion_warnings.is_empty());

        // Simulate final warning time - 1 day before deletion
        simulator.simulate_time_passage(Duration::days(6)); // Now 1 day left

        simulator.process_deletion_timeline_notifications();

        // Should have final warning
        let final_notifications = simulator.get_notifications(&user.id);
        let final_warnings: Vec<_> = final_notifications.iter()
            .filter(|n| n.notification_type == NotificationType::FinalWarning)
            .collect();
        assert!(!final_warnings.is_empty());
    }

    #[test]
    fn test_multi_factor_account_recovery() {
        let mut simulator = AccountDeactivationSimulator::new();
        let user = simulator.create_test_user("mfa@example.com".to_string(), "mfauser".to_string());

        // Set up user with high security requirements
        simulator.set_compliance_flag(&user.id, "HIGH_SECURITY".to_string());

        // Deactivate account
        simulator.deactivate_account(&user.id, &user.id, Some("MFA test".to_string()), "192.168.1.100".to_string()).unwrap();

        // First recovery step - email token
        let email_recovery = simulator.attempt_account_recovery(
            &user.id,
            RecoveryMethod::EmailToken,
            "192.168.1.100".to_string(),
            "SecureBrowser/1.0".to_string()
        ).unwrap();
        assert!(email_recovery.success);

        // Second recovery step - identity verification
        let identity_recovery = simulator.attempt_account_recovery(
            &user.id,
            RecoveryMethod::IdentityVerification,
            "192.168.1.100".to_string(),
            "SecureBrowser/1.0".to_string()
        ).unwrap();
        assert!(identity_recovery.success);

        // Now reactivation should be possible
        let token = simulator.request_reactivation(&user.id, "192.168.1.100".to_string()).unwrap();
        let reactivated_user = simulator.reactivate_account(&token, "192.168.1.100".to_string()).unwrap();
        assert_eq!(reactivated_user.status, UserStatus::Active);

        // Verify recovery audit trail
        let recovery_attempts = simulator.get_recovery_attempts(&user.id);
        assert_eq!(recovery_attempts.len(), 2);
        assert!(recovery_attempts.iter().any(|r| r.method == RecoveryMethod::EmailToken));
        assert!(recovery_attempts.iter().any(|r| r.method == RecoveryMethod::IdentityVerification));
    }
}