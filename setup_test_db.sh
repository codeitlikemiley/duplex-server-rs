#!/bin/bash

# Setup script for integration test database
set -e

echo "Setting up test database environment..."

# Get current username for database connection
USERNAME=${USER:-${USERNAME:-postgres}}

# Set default database URL for tests
export DATABASE_URL="postgres://${USERNAME}@localhost:5432/coqrs_test"

# Check if PostgreSQL is running
if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
    echo "Error: PostgreSQL is not running on localhost:5432"
    echo "Please start PostgreSQL and try again"
    exit 1
fi

# Create test database if it doesn't exist
psql postgres -c "CREATE DATABASE coqrs_test OWNER ${USERNAME};" 2>/dev/null || {
    echo "Test database already exists or creating failed, continuing..."
}

# Run migrations to ensure schema is up to date
echo "Running database migrations..."
cd "$(dirname "$0")"
cargo install sqlx-cli --features postgres 2>/dev/null || echo "sqlx-cli already installed"
sqlx database create 2>/dev/null || echo "Database already exists"
sqlx migrate run

echo "Test database environment setup complete!"
echo "You can now run integration tests with: cargo test --test database_integration_tests"