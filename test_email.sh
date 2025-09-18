#!/bin/bash

# Email Testing Script for Quake Application
# This script tests all email functionality with different providers

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://localhost:80"
TEST_USER="testuser_$(date +%s)"
TEST_EMAIL="test_$(date +%s)@example.com"
TEST_PASSWORD="SecurePass123!"

echo -e "${BLUE}🚀 Quake Email Testing Suite${NC}"
echo "================================="

# Function to print status
print_status() {
    echo -e "${BLUE}➜${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Function to check if server is running
check_server() {
    print_status "Checking if Quake server is running..."
    if curl -s "$BASE_URL/health" > /dev/null; then
        print_success "Server is running at $BASE_URL"
    else
        print_error "Server is not running. Please start with 'cargo run'"
        exit 1
    fi
}

# Function to test user registration (triggers email verification)
test_registration() {
    print_status "Testing user registration (should trigger verification email)..."

    RESPONSE=$(curl -s -w "%{http_code}" -X POST "$BASE_URL/api/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$TEST_USER\",
            \"email\": \"$TEST_EMAIL\",
            \"password\": \"$TEST_PASSWORD\",
            \"first_name\": \"Test\",
            \"last_name\": \"User\"
        }")

    HTTP_CODE="${RESPONSE: -3}"
    BODY="${RESPONSE%???}"

    if [ "$HTTP_CODE" -eq 201 ] || [ "$HTTP_CODE" -eq 200 ]; then
        print_success "User registration successful (HTTP $HTTP_CODE)"
        echo "Response: $BODY"
        echo
        print_warning "Check your console/email for verification message!"
        return 0
    else
        print_error "Registration failed (HTTP $HTTP_CODE)"
        echo "Response: $BODY"
        return 1
    fi
}

# Function to test password reset (triggers reset email)
test_password_reset() {
    print_status "Testing password reset (should trigger reset email)..."

    RESPONSE=$(curl -s -w "%{http_code}" -X POST "$BASE_URL/api/reset-password" \
        -H "Content-Type: application/json" \
        -d "{\"email\": \"$TEST_EMAIL\"}")

    HTTP_CODE="${RESPONSE: -3}"
    BODY="${RESPONSE%???}"

    if [ "$HTTP_CODE" -eq 200 ]; then
        print_success "Password reset request successful (HTTP $HTTP_CODE)"
        echo "Response: $BODY"
        echo
        print_warning "Check your console/email for reset message!"
        return 0
    else
        print_error "Password reset failed (HTTP $HTTP_CODE)"
        echo "Response: $BODY"
        return 1
    fi
}

# Function to show email configuration
show_email_config() {
    print_status "Current email configuration:"
    echo "EMAIL_SERVICE: ${EMAIL_SERVICE:-console (default)}"

    case "${EMAIL_SERVICE:-console}" in
        "console")
            echo "EMAIL_LOG_FULL_CONTENT: ${EMAIL_LOG_FULL_CONTENT:-false (default)}"
            print_warning "Using console mode - emails will be printed to application logs"
            ;;
        "smtp")
            echo "SMTP_HOST: ${SMTP_HOST:-not set}"
            echo "SMTP_PORT: ${SMTP_PORT:-587 (default)}"
            echo "SMTP_USERNAME: ${SMTP_USERNAME:-not set}"
            echo "SMTP_FROM_EMAIL: ${SMTP_FROM_EMAIL:-noreply@quake.app (default)}"
            echo "SMTP_USE_TLS: ${SMTP_USE_TLS:-true (default)}"
            ;;
        "sendgrid")
            echo "SENDGRID_FROM_EMAIL: ${SENDGRID_FROM_EMAIL:-noreply@quake.app (default)}"
            echo "SENDGRID_FROM_NAME: ${SENDGRID_FROM_NAME:-Quake App (default)}"
            if [ -n "$SENDGRID_API_KEY" ]; then
                echo "SENDGRID_API_KEY: Set (${#SENDGRID_API_KEY} characters)"
            else
                print_error "SENDGRID_API_KEY: Not set"
            fi
            ;;
    esac
    echo
}

# Function to run comprehensive tests
run_comprehensive_test() {
    echo -e "${BLUE}📧 Running comprehensive email tests...${NC}"
    echo

    show_email_config
    check_server

    echo
    print_status "Testing with user: $TEST_USER"
    print_status "Testing with email: $TEST_EMAIL"
    echo

    # Test registration
    if test_registration; then
        echo

        # Wait a moment for registration to process
        sleep 2

        # Test password reset
        test_password_reset
    else
        print_error "Registration test failed, skipping password reset test"
    fi

    echo
    print_status "Test completed!"
    echo
    print_warning "Manual verification required:"
    echo "1. Check console output for email content (if using console mode)"
    echo "2. Check your email inbox (if using SMTP/SendGrid)"
    echo "3. Use the verification token to test email verification endpoint"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [command]"
    echo
    echo "Commands:"
    echo "  test          Run comprehensive email tests"
    echo "  register      Test user registration only"
    echo "  reset         Test password reset only"
    echo "  config        Show current email configuration"
    echo "  help          Show this help message"
    echo
    echo "Environment variables:"
    echo "  EMAIL_SERVICE         console|smtp|sendgrid (default: console)"
    echo "  EMAIL_LOG_FULL_CONTENT true|false (default: false, console mode only)"
    echo "  BASE_URL              Server URL (default: http://localhost:80)"
    echo
    echo "Examples:"
    echo "  # Test with console mode (development)"
    echo "  EMAIL_SERVICE=console EMAIL_LOG_FULL_CONTENT=true $0 test"
    echo
    echo "  # Test with SMTP"
    echo "  EMAIL_SERVICE=smtp SMTP_HOST=smtp.gmail.com SMTP_USERNAME=user@gmail.com $0 test"
    echo
    echo "  # Test with SendGrid"
    echo "  EMAIL_SERVICE=sendgrid SENDGRID_API_KEY=SG.your-key $0 test"
}

# Main script logic
case "${1:-test}" in
    "test")
        run_comprehensive_test
        ;;
    "register")
        show_email_config
        check_server
        test_registration
        ;;
    "reset")
        show_email_config
        check_server
        test_password_reset
        ;;
    "config")
        show_email_config
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        print_error "Unknown command: $1"
        echo
        show_usage
        exit 1
        ;;
esac