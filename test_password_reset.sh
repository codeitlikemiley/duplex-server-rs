#!/bin/bash

# Test Password Reset Flow
# This script tests the complete password reset workflow

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔑 Testing Password Reset Flow"
echo "==============================="

# Configuration
BASE_URL="${BASE_URL:-http://localhost:80}"
EMAIL="test_reset@example.com"
USERNAME="resetuser"
PASSWORD="OldPassword123!"
NEW_PASSWORD="NewPassword456!"

echo "Base URL: $BASE_URL"
echo ""

# Step 1: Register a new user
echo -e "${YELLOW}1. Registering test user${NC}"
echo "------------------------------"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"$USERNAME\",
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\",
    \"first_name\": \"Reset\",
    \"last_name\": \"Test\"
  }")

echo "Response: $REGISTER_RESPONSE"

# Check if registration was successful
if echo "$REGISTER_RESPONSE" | grep -q "Registration successful"; then
    echo -e "${GREEN}✅ Registration successful${NC}"
else
    echo -e "${RED}❌ Registration failed${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}2. Getting Verification Token from Console${NC}"
echo "---------------------------------------------"
echo "Check the server console for the verification token."
echo "Look for: 📧 VERIFICATION TOKEN: <TOKEN>"
echo ""
read -p "Enter the verification token: " VERIFICATION_TOKEN
read -p "Enter the user ID (UUID): " USER_ID

# Step 3: Verify email
echo ""
echo -e "${YELLOW}3. Verifying email${NC}"
echo "--------------------"
VERIFY_RESPONSE=$(curl -s -X POST "$BASE_URL/api/verify-email" \
  -H "Content-Type: application/json" \
  -d "{
    \"user_id\": \"$USER_ID\",
    \"verification_token\": \"$VERIFICATION_TOKEN\"
  }")

echo "Response: $VERIFY_RESPONSE"

if echo "$VERIFY_RESPONSE" | grep -q "Email verified successfully"; then
    echo -e "${GREEN}✅ Email verified${NC}"
else
    echo -e "${RED}❌ Email verification failed${NC}"
    exit 1
fi

# Step 4: Test login with old password
echo ""
echo -e "${YELLOW}4. Testing login with old password${NC}"
echo "-------------------------------------"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\"
  }")

if echo "$LOGIN_RESPONSE" | grep -q "token"; then
    echo -e "${GREEN}✅ Login successful with old password${NC}"
    TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*' | sed 's/"token":"//')
else
    echo -e "${RED}❌ Login failed${NC}"
    exit 1
fi

# Step 5: Request password reset
echo ""
echo -e "${YELLOW}5. Requesting password reset${NC}"
echo "-------------------------------"
RESET_REQUEST_RESPONSE=$(curl -s -X POST "$BASE_URL/api/request-password-reset" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\"
  }")

echo "Response: $RESET_REQUEST_RESPONSE"

if echo "$RESET_REQUEST_RESPONSE" | grep -q "password reset link has been sent"; then
    echo -e "${GREEN}✅ Password reset requested${NC}"
else
    echo -e "${RED}❌ Password reset request failed${NC}"
    exit 1
fi

# Step 6: Get reset token from console
echo ""
echo -e "${YELLOW}6. Getting Reset Token from Console${NC}"
echo "--------------------------------------"
echo "Check the server console for the password reset token."
echo "Look for: 📧 VERIFICATION LINK: ...token=<TOKEN>"
echo ""
read -p "Enter the password reset token: " RESET_TOKEN

# Step 7: Reset password with token
echo ""
echo -e "${YELLOW}7. Resetting password${NC}"
echo "-----------------------"
RESET_RESPONSE=$(curl -s -X POST "$BASE_URL/api/reset-password" \
  -H "Content-Type: application/json" \
  -d "{
    \"user_id\": \"$USER_ID\",
    \"token\": \"$RESET_TOKEN\",
    \"new_password\": \"$NEW_PASSWORD\"
  }")

echo "Response: $RESET_RESPONSE"

if echo "$RESET_RESPONSE" | grep -q "Password has been reset successfully"; then
    echo -e "${GREEN}✅ Password reset successful${NC}"
else
    echo -e "${RED}❌ Password reset failed${NC}"
    exit 1
fi

# Step 8: Test login with old password (should fail)
echo ""
echo -e "${YELLOW}8. Testing login with old password (should fail)${NC}"
echo "---------------------------------------------------"
OLD_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\"
  }")

if echo "$OLD_LOGIN_RESPONSE" | grep -q "token"; then
    echo -e "${RED}❌ Old password still works (security issue!)${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Old password no longer works${NC}"
fi

# Step 9: Test login with new password
echo ""
echo -e "${YELLOW}9. Testing login with new password${NC}"
echo "-------------------------------------"
NEW_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$NEW_PASSWORD\"
  }")

if echo "$NEW_LOGIN_RESPONSE" | grep -q "token"; then
    echo -e "${GREEN}✅ Login successful with new password${NC}"
    NEW_TOKEN=$(echo "$NEW_LOGIN_RESPONSE" | grep -o '"token":"[^"]*' | sed 's/"token":"//')
    echo "New JWT Token: ${NEW_TOKEN:0:50}..."
else
    echo -e "${RED}❌ Login failed with new password${NC}"
    exit 1
fi

# Step 10: Verify old sessions are invalidated
echo ""
echo -e "${YELLOW}10. Verifying old sessions are invalidated${NC}"
echo "--------------------------------------------"
if [ ! -z "$TOKEN" ]; then
    PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/profile" \
      -H "Authorization: Bearer $TOKEN")

    if echo "$PROFILE_RESPONSE" | grep -q "user_id"; then
        echo -e "${RED}❌ Old session still valid (security issue!)${NC}"
        exit 1
    else
        echo -e "${GREEN}✅ Old session invalidated${NC}"
    fi
fi

echo ""
echo "==============================="
echo -e "${GREEN}✅ All password reset tests passed!${NC}"
echo ""
echo "Summary:"
echo "- User registered successfully"
echo "- Email verified"
echo "- Password reset requested"
echo "- Password changed successfully"
echo "- Old password no longer works"
echo "- New password works"
echo "- Old sessions invalidated"
echo ""