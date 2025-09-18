use sqlx::{Pool, Postgres, PgPool};
use std::sync::Once;
use uuid::Uuid;
use chrono::Utc;
use tokio::sync::Mutex;
use std::sync::Arc;

use coqrs::{models::{User, UserProfile, UserStatus}, services::{UserService, PasswordService}};
use coqrs::PostgreSQL;
use coqrs::infrastructure::logger::init_logger;

static INIT: Once = Once::new();

/// Shared test database pool to avoid creating multiple connections
static TEST_DB_POOL: Mutex<Option<Arc<PgPool>>> = Mutex::const_new(None);

/// Initialize the test environment (logging, database setup, etc.)
pub fn init_test_env() {
    INIT.call_once(|| {
        init_logger();

        // Set default database URL for tests if not already set
        if std::env::var("DATABASE_URL").is_err() {
            let username = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
            let default_url = format!("postgres://{}@localhost:5432/coqrs_test", username);
            unsafe {
                std::env::set_var("DATABASE_URL", &default_url);
            }
        }
    });
}

/// Setup function to ensure database environment is ready for tests
pub async fn setup_database_environment() -> Result<(), Box<dyn std::error::Error>> {
    init_test_env();

    let username = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
    let base_url = format!("postgres://{}@localhost:5432", username);
    let admin_pool = PgPool::connect(&base_url).await?;

    // Ensure the main test database exists
    sqlx::query("CREATE DATABASE coqrs_test")
        .execute(&admin_pool)
        .await
        .ok(); // Ignore error if it already exists

    admin_pool.close().await;

    // Connect to test database and run migrations if needed
    let test_url = format!("postgres://{}@localhost:5432/coqrs_test", username);
    let test_pool = PgPool::connect(&test_url).await?;

    // Run migrations to ensure schema is up to date
    sqlx::migrate!("./migrations").run(&test_pool).await?;

    test_pool.close().await;

    Ok(())
}

/// Database test configuration
pub struct TestConfig {
    pub database_url: String,
    pub test_db_name: String,
}

impl TestConfig {
    pub fn new() -> Self {
        let username = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        let base_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://{}@localhost:5432", username));

        let test_db_name = format!("test_quake_{}", Uuid::new_v4().simple());

        Self {
            database_url: base_url,
            test_db_name,
        }
    }

    pub fn test_database_url(&self) -> String {
        // Parse the base URL and replace the database name
        if let Some(pos) = self.database_url.rfind('/') {
            format!("{}/{}", &self.database_url[..pos], self.test_db_name)
        } else {
            format!("{}/{}", self.database_url, self.test_db_name)
        }
    }

    pub fn base_database_url(&self) -> String {
        // Return the base URL without the database name
        if let Some(pos) = self.database_url.rfind('/') {
            self.database_url[..pos].to_string()
        } else {
            self.database_url.clone()
        }
    }
}

/// Test database manager for integration tests
pub struct TestDatabase {
    pub pool: PgPool,
    pub config: TestConfig,
    pub postgres: PostgreSQL,
}

impl TestDatabase {
    /// Create a new test database instance with clean slate
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        init_test_env();

        let config = TestConfig::new();

        // Connect to the base database (not the test database)
        let admin_pool = PgPool::connect(&config.base_database_url()).await?;

        // Drop existing test database if it exists
        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", config.test_db_name))
            .execute(&admin_pool)
            .await?;

        // Create the test database
        let username = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        sqlx::query(&format!("CREATE DATABASE \"{}\" OWNER {}", config.test_db_name, username))
            .execute(&admin_pool)
            .await?;

        admin_pool.close().await;

        // Connect to the test database
        let pool = PgPool::connect(&config.test_database_url()).await?;

        // Run migrations to set up schema
        sqlx::migrate!("./migrations").run(&pool).await?;

        let postgres = PostgreSQL::new(pool.clone());

        Ok(Self {
            pool,
            config,
            postgres,
        })
    }

    /// Create a shared test database (reused across tests for performance)
    pub async fn shared() -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let mut pool_guard = TEST_DB_POOL.lock().await;

        match pool_guard.as_ref() {
            Some(pool) => {
                // Reuse existing pool but clean it
                let test_db = TestDatabase {
                    pool: (**pool).clone(),
                    config: TestConfig::new(), // This is just for the struct, not used
                    postgres: PostgreSQL::new((**pool).clone()),
                };
                test_db.cleanup().await?;
                Ok(Arc::new(test_db))
            }
            None => {
                // Create new pool
                let test_db = Arc::new(Self::new().await?);
                *pool_guard = Some(test_db.pool.clone().into());
                Ok(test_db)
            }
        }
    }

    /// Clean up test data (truncate tables)
    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        // Disable foreign key constraints temporarily
        sqlx::query("SET session_replication_role = replica")
            .execute(&self.pool)
            .await?;

        // Truncate all tables
        let tables = vec![
            "user_profiles",
            "password_reset_tokens",
            "user_sessions",
            "email_verification_tokens",
            "user_activities",
            "auth_rate_limits",
            "account_lockouts",
            "user_roles",
            "role_permissions",
            "roles",
            "permissions",
            "users",
        ];

        for table in tables {
            sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table))
                .execute(&self.pool)
                .await
                .ok(); // Ignore errors for tables that don't exist
        }

        // Re-enable foreign key constraints
        sqlx::query("SET session_replication_role = DEFAULT")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Drop the test database
    pub async fn drop_database(self) -> Result<(), Box<dyn std::error::Error>> {
        let db_name = self.config.test_db_name.clone();

        // Close the connection pool
        self.pool.close().await;

        // Connect to admin database to drop test database
        let admin_pool = PgPool::connect(&self.config.database_url).await?;

        // Terminate existing connections to the test database
        sqlx::query(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            db_name
        ))
        .execute(&admin_pool)
        .await?;

        // Drop the test database
        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", db_name))
            .execute(&admin_pool)
            .await?;

        admin_pool.close().await;

        Ok(())
    }
}

/// Test data factory for creating test users and profiles
pub struct TestDataFactory {
    pub db: Arc<TestDatabase>,
}

impl TestDataFactory {
    pub fn new(db: Arc<TestDatabase>) -> Self {
        Self { db }
    }

    /// Create a test user in the database
    pub async fn create_user(&self, username: &str, email: &str) -> Result<User, sqlx::Error> {
        let user = User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(), // Mock hash
            email_verified: false,
            status: UserStatus::PendingVerification,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        };

        self.db.postgres.save_user(user.clone()).await?;
        Ok(user)
    }

    /// Create a verified active user
    pub async fn create_active_user(&self, username: &str, email: &str) -> Result<User, sqlx::Error> {
        let user = User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            password_hash: "$argon2id$v=19$m=4096,t=3,p=1$salt$hash".to_string(),
            email_verified: true,
            status: UserStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: Some(Utc::now()),
        };

        self.db.postgres.save_user(user.clone()).await?;
        Ok(user)
    }

    /// Create a user profile in the database
    pub async fn create_profile(&self, user_id: Uuid, first_name: Option<&str>, last_name: Option<&str>) -> Result<UserProfile, sqlx::Error> {
        let profile = UserProfile {
            user_id,
            first_name: first_name.map(|s| s.to_string()),
            last_name: last_name.map(|s| s.to_string()),
            bio: None,
            avatar_url: None,
            website: None,
            location: None,
            preferences: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.db.postgres.save_profile(profile.clone()).await?;
        Ok(profile)
    }

    /// Create multiple test users
    pub async fn create_users(&self, count: usize) -> Result<Vec<User>, sqlx::Error> {
        let mut users = Vec::new();

        for i in 0..count {
            let user = self.create_user(
                &format!("user{}", i),
                &format!("user{}@example.com", i),
            ).await?;
            users.push(user);
        }

        Ok(users)
    }
}

/// Integration test context with all necessary components
pub struct IntegrationTestContext {
    pub db: Arc<TestDatabase>,
    pub data_factory: TestDataFactory,
    pub user_service: UserService,
    pub password_service: PasswordService,
}

impl IntegrationTestContext {
    /// Create a new integration test context
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db = TestDatabase::shared().await?;
        let data_factory = TestDataFactory::new(db.clone());

        // Initialize services with real database connections
        let user_service = UserService::new(
            db.postgres.clone(),
            // TODO: Replace with a proper mock or dummy sender for tests
            tokio::sync::mpsc::channel(1).0,
        );

        let password_service = PasswordService::new(db.postgres.clone());

        Ok(Self {
            db,
            data_factory,
            user_service,
            password_service,
        })
    }

    /// Reset the test environment
    pub async fn reset(&self) -> Result<(), sqlx::Error> {
        self.db.cleanup().await
    }
}

/// Utility macros for integration tests
#[macro_export]
macro_rules! integration_test {
    ($test_name:ident, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let ctx = IntegrationTestContext::new().await.expect("Failed to create test context");
            ctx.reset().await.expect("Failed to reset test environment");

            let result = $test_body(ctx).await;

            match result {
                Ok(_) => {},
                Err(e) => panic!("Test failed: {:?}", e),
            }
        }
    };
}

/// Database transaction testing helpers
pub struct TransactionTestHelper {
    pub pool: PgPool,
}

impl TransactionTestHelper {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run a test within a database transaction that rolls back
    pub async fn with_rollback<F, Fut, T>(&self, test_fn: F) -> Result<T, sqlx::Error>
    where
        F: FnOnce(sqlx::Transaction<'_, Postgres>) -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let tx = self.pool.begin().await?;

        let result = test_fn(tx).await;

        // Always rollback to ensure test isolation
        // tx.rollback().await?;

        result
    }
}