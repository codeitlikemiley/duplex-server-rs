# Integration Tests

This directory contains comprehensive integration tests for the Quake user management system.

## Test Structure

### Database Integration Tests (`database_integration_tests.rs`)
- **PostgreSQL Repository Testing**: Real database CRUD operations
- **Transaction Testing**: Database transaction rollback and commit behavior
- **Constraint Validation**: Unique constraints and foreign key relationships
- **Performance Testing**: Bulk operations and query performance
- **Connection Pool Management**: Database connection lifecycle

### HTTP API Integration Tests (`http_api_integration_tests.rs`)
- **REST Endpoint Testing**: User creation, retrieval, and management
- **Request Validation**: Input validation and error handling
- **Content Type Handling**: JSON parsing and response formatting
- **Security Headers**: Security middleware validation
- **Rate Limiting**: API rate limiting behavior
- **Pagination**: Large dataset pagination logic

### gRPC Service Integration Tests (`grpc_service_integration_tests.rs`)
- **Service Method Testing**: All gRPC service endpoints
- **Protocol Buffer Serialization**: Message serialization/deserialization
- **Error Handling**: gRPC status codes and error messages
- **Metadata Handling**: Custom headers and request metadata
- **Concurrent Operations**: Multiple simultaneous gRPC calls
- **Large Payload Handling**: Performance with large messages

### End-to-End Workflow Tests (`end_to_end_workflow_tests.rs`)
- **Complete User Registration**: Registration → Verification → Login → Profile Setup
- **Password Reset Flow**: Request → Token Generation → Reset → New Login
- **Account Lifecycle**: Creation → Usage → Deactivation → Reactivation → Suspension
- **Concurrent User Workflows**: Multiple users going through flows simultaneously
- **Error Recovery**: Failed operations and recovery scenarios

### Test Utilities (`integration_test_helpers.rs`)
- **Test Database Management**: Isolated test databases with automatic cleanup
- **Data Factory**: Consistent test data generation
- **Integration Test Context**: Pre-configured test environment
- **Transaction Helpers**: Database transaction testing utilities

## Prerequisites

### Database Setup
```bash
# Start PostgreSQL (Docker example)
docker run --name postgres-test \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=postgres \
  -p 5432:5432 \
  -d postgres:15

# Set environment variable
export DATABASE_URL="postgres://postgres:password@localhost:5432"
```

### Environment Variables
Required environment variables for testing:
```bash
export DATABASE_URL="postgres://postgres:password@localhost:5432"
export RUST_LOG=debug  # Optional: Enable debug logging
```

## Running Tests

### All Integration Tests
```bash
# Run all integration tests
cargo test --test database_integration_tests
cargo test --test http_api_integration_tests
cargo test --test grpc_service_integration_tests
cargo test --test end_to_end_workflow_tests
```

### Specific Test Categories
```bash
# Database tests only
cargo test --test database_integration_tests

# HTTP API tests only
cargo test --test http_api_integration_tests

# gRPC tests only
cargo test --test grpc_service_integration_tests

# End-to-end workflow tests only
cargo test --test end_to_end_workflow_tests
```

### Specific Test Functions
```bash
# Run specific test
cargo test --test database_integration_tests test_user_repository_crud_operations

# Run tests with output
cargo test --test end_to_end_workflow_tests -- --nocapture
```

### Parallel vs Sequential Execution
```bash
# Run tests sequentially (recommended for database tests)
cargo test --test database_integration_tests -- --test-threads=1

# Run tests in parallel (default)
cargo test --test database_integration_tests
```

## Test Database Management

### Automatic Database Creation
Each test creates its own isolated PostgreSQL database:
- Database name format: `test_quake_{uuid}`
- Automatic migration execution
- Automatic cleanup after tests

### Manual Database Cleanup
If tests fail unexpectedly, you may need to clean up test databases:
```sql
-- Connect to PostgreSQL and list test databases
\l

-- Drop test databases (replace with actual names)
DROP DATABASE "test_quake_abc123...";
```

### Shared vs Isolated Databases
- **Isolated**: Each test gets its own database (slower but completely isolated)
- **Shared**: Tests share a database but clean up between tests (faster but potential for interference)

The test suite uses **shared databases with cleanup** for performance while maintaining isolation.

## Test Configuration

### Test Timeouts
Long-running tests have appropriate timeouts:
```rust
#[tokio::test]
#[timeout(Duration::from_secs(30))]
async fn test_long_running_operation() {
    // Test implementation
}
```

### Logging Configuration
Enable detailed logging for test debugging:
```bash
export RUST_LOG=debug,sqlx=info,tower_http=debug
cargo test -- --nocapture
```

### Mock vs Real Services
- **Database**: Real PostgreSQL connections
- **HTTP Clients**: Mock HTTP clients for external APIs
- **Email**: Mock email service (no actual emails sent)
- **Authentication**: Real JWT token generation and validation

## Performance Considerations

### Test Execution Time
- Database tests: ~30-60 seconds (includes database creation/cleanup)
- HTTP API tests: ~10-20 seconds (conceptual, no real HTTP server)
- gRPC tests: ~20-40 seconds (includes gRPC server startup)
- End-to-end tests: ~60-120 seconds (complete workflows)

### Resource Usage
- **Memory**: ~100-200MB per test database
- **CPU**: Moderate during test execution
- **Network**: Local connections only
- **Disk**: Temporary databases (auto-cleaned)

## Troubleshooting

### Common Issues

#### Database Connection Errors
```
Error: Failed to connect to database
```
**Solution**: Ensure PostgreSQL is running and `DATABASE_URL` is set correctly.

#### Permission Errors
```
Error: permission denied to create database
```
**Solution**: Ensure the database user has `CREATEDB` privileges.

#### Port Conflicts
```
Error: Address already in use
```
**Solution**: For gRPC tests, ensure no other services are using the test ports.

#### Test Timeouts
```
Error: test timed out
```
**Solution**: Increase timeout or check for deadlocks in test code.

### Debug Mode
Enable detailed debugging:
```bash
export RUST_LOG=debug
export RUST_BACKTRACE=1
cargo test --test end_to_end_workflow_tests -- --nocapture
```

### Test Data Inspection
To inspect test data during debugging:
```rust
// Add this to your test before cleanup
println!("Test database: {}", test_db.config.test_database_url());
std::thread::sleep(std::time::Duration::from_secs(30)); // Pause for inspection
```

## Continuous Integration

### CI Configuration
For GitHub Actions or similar CI systems:
```yaml
name: Integration Tests
services:
  postgres:
    image: postgres:15
    env:
      POSTGRES_PASSWORD: password
    options: >-
      --health-cmd pg_isready
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5

env:
  DATABASE_URL: postgres://postgres:password@localhost:5432

steps:
  - name: Run Integration Tests
    run: |
      cargo test --test database_integration_tests
      cargo test --test http_api_integration_tests
      cargo test --test grpc_service_integration_tests
      cargo test --test end_to_end_workflow_tests
```

### Test Coverage
Integration tests provide coverage for:
- ✅ Database layer (repositories, transactions)
- ✅ Service layer (business logic)
- ✅ API layer (HTTP/gRPC endpoints)
- ✅ End-to-end workflows (complete user journeys)
- ✅ Error handling and recovery
- ✅ Concurrent operations
- ✅ Performance characteristics

## Best Practices

1. **Isolation**: Each test should be independent and not rely on other tests
2. **Cleanup**: Always clean up test data, even if tests fail
3. **Realistic Data**: Use realistic test data that matches production scenarios
4. **Error Testing**: Test both success and failure scenarios
5. **Performance**: Include performance-oriented tests for critical paths
6. **Documentation**: Document complex test scenarios and expected behaviors

## Contributing

When adding new integration tests:

1. **Choose the Right Test File**:
   - Database operations → `database_integration_tests.rs`
   - API endpoints → `http_api_integration_tests.rs` or `grpc_service_integration_tests.rs`
   - Complete workflows → `end_to_end_workflow_tests.rs`

2. **Use Test Utilities**: Leverage `integration_test_helpers.rs` for consistency

3. **Follow Naming Conventions**:
   - Test functions: `test_specific_functionality`
   - Test data: Use descriptive names that indicate the test scenario

4. **Include Error Cases**: Test both success and failure scenarios

5. **Add Documentation**: Include comments explaining complex test logic

6. **Consider Performance**: Be mindful of test execution time and resource usage