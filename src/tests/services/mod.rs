//! Tests for application services

#[cfg(test)]
pub mod user_service_tests;
#[cfg(test)]
pub mod authentication_service_tests;
#[cfg(test)]
pub mod authorization_service_tests;
#[cfg(test)]
pub mod password_service_tests;
#[cfg(test)]
pub mod jwt_service_tests;
#[cfg(test)]
pub mod session_service_tests;
#[cfg(test)]
pub mod email_verification_service_tests;
#[cfg(test)]
pub mod rate_limiting_service_tests;
#[cfg(test)]
pub mod role_permission_tests;