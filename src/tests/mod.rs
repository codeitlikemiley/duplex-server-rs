//! Test modules for the application
//!
//! This module organizes all tests for the application,
//! including unit tests, integration tests, and service tests.

#[cfg(test)]
pub mod models;
#[cfg(test)]
pub mod services;
#[cfg(test)]
pub mod repositories;
#[cfg(test)]
pub mod security;
#[cfg(test)]
pub mod validation;
#[cfg(test)]
pub mod controllers;
#[cfg(test)]
pub mod integration;