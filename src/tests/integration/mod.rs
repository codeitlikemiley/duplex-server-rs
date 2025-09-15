//! Integration tests

#[cfg(test)]
pub mod user_flow_tests;
#[cfg(test)]
pub mod user_registration_flow_tests;
#[cfg(test)]
pub mod user_login_session_tests;
#[cfg(test)]
pub mod password_reset_flow_tests;
#[cfg(test)]
pub mod profile_update_tests;
#[cfg(test)]
pub mod account_deactivation_tests;
#[cfg(test)]
pub mod email_verification_tests;
#[cfg(test)]
pub mod session_management_tests;
#[cfg(test)]
pub mod two_factor_auth_tests;
#[cfg(test)]
pub mod account_lockout_tests;
#[cfg(test)]
pub mod role_permission_tests;
#[cfg(test)]
pub mod api_consistency_tests;
#[cfg(test)]
pub mod database_transaction_tests;
#[cfg(test)]
pub mod error_recovery_tests;
#[cfg(test)]
pub mod performance_load_tests;
#[cfg(test)]
pub mod security_vulnerability_tests;
#[cfg(test)]
pub mod data_consistency_tests;
#[cfg(test)]
pub mod admin_management_tests;