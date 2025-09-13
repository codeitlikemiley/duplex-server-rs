use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    PostgreSQL,
    commands::{CommandMessage, CreateUser, Login, RegisterUser, VerifyEmail, send_command},
    errors::AppError,
    infrastructure::auth::JwtService,
    models::{User, UserProfile, UserStatus},
    repositories::{UserRepository, UserProfileRepository},
};