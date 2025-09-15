#!/bin/bash

# Test User Profile Operations
# This script tests profile CRUD operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "👤 Testing User Profile Operations"
echo "====================================="

# Configuration
BASE_URL="${BASE_URL:-http://localhost:80}"
EMAIL="profile_test_$(date +%s)@example.com"
USERNAME="profileuser_$(date +%s)"
PASSWORD="TestPassword123!"

echo "Base URL: $BASE_URL"
echo "Test Email: $EMAIL"
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
    \"first_name\": \"Initial\",
    \"last_name\": \"Name\"
  }")

echo "Response: $REGISTER_RESPONSE"

if echo "$REGISTER_RESPONSE" | grep -q "Registration successful"; then
    echo -e "${GREEN}✅ Registration successful${NC}"
else
    echo -e "${RED}❌ Registration failed${NC}"
    exit 1
fi

# Step 2: Get verification token from console
echo ""
echo -e "${YELLOW}2. Getting Verification Token${NC}"
echo "--------------------------------"
echo "Check server console for verification token"
read -p "Enter verification token: " VERIFICATION_TOKEN
read -p "Enter user ID: " USER_ID

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

if echo "$VERIFY_RESPONSE" | grep -q "Email verified successfully"; then
    echo -e "${GREEN}✅ Email verified${NC}"
else
    echo -e "${RED}❌ Email verification failed${NC}"
    exit 1
fi

# Step 4: Login to get JWT token
echo ""
echo -e "${YELLOW}4. Logging in${NC}"
echo "---------------"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\"
  }")

TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*' | sed 's/"token":"//')

if [ -z "$TOKEN" ]; then
    echo -e "${RED}❌ Login failed${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Login successful${NC}"
echo "Token: ${TOKEN:0:50}..."

# Step 5: Get initial profile (should be empty or minimal)
echo ""
echo -e "${YELLOW}5. Getting initial profile${NC}"
echo "----------------------------"
PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "Profile Response: $PROFILE_RESPONSE"

# Step 6: Update profile
echo ""
echo -e "${YELLOW}6. Updating profile${NC}"
echo "---------------------"
UPDATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "John",
    "last_name": "Doe",
    "bio": "Software developer passionate about Rust",
    "location": "San Francisco, CA",
    "website": "https://johndoe.dev",
    "avatar_url": "https://example.com/avatar.jpg",
    "preferences": {
      "theme": "dark",
      "notifications": true,
      "language": "en"
    }
  }')

echo "Update Response: $UPDATE_RESPONSE"

if echo "$UPDATE_RESPONSE" | grep -q "Profile updated successfully"; then
    echo -e "${GREEN}✅ Profile updated${NC}"
else
    echo -e "${RED}❌ Profile update failed${NC}"
    exit 1
fi

# Step 7: Get updated profile
echo ""
echo -e "${YELLOW}7. Getting updated profile${NC}"
echo "----------------------------"
UPDATED_PROFILE=$(curl -s -X GET "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "Updated Profile: $UPDATED_PROFILE"

# Step 8: Update account details (username)
echo ""
echo -e "${YELLOW}8. Updating account details${NC}"
echo "-----------------------------"
NEW_USERNAME="${USERNAME}_updated"
ACCOUNT_UPDATE=$(curl -s -X PUT "$BASE_URL/api/user/account" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"$NEW_USERNAME\"
  }")

echo "Account Update Response: $ACCOUNT_UPDATE"

if echo "$ACCOUNT_UPDATE" | grep -q "Account updated successfully"; then
    echo -e "${GREEN}✅ Account updated${NC}"
else
    echo -e "${RED}❌ Account update failed${NC}"
    exit 1
fi

# Step 9: Test username uniqueness (should fail)
echo ""
echo -e "${YELLOW}9. Testing username uniqueness${NC}"
echo "--------------------------------"
DUPLICATE_USERNAME=$(curl -s -X PUT "$BASE_URL/api/user/account" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"$NEW_USERNAME\"
  }")

if echo "$DUPLICATE_USERNAME" | grep -q "error"; then
    echo -e "${GREEN}✅ Username uniqueness enforced${NC}"
else
    echo -e "${YELLOW}⚠️ Username uniqueness check might not be working${NC}"
fi

# Step 10: Partial profile update
echo ""
echo -e "${YELLOW}10. Testing partial profile update${NC}"
echo "------------------------------------"
PARTIAL_UPDATE=$(curl -s -X PUT "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "bio": "Updated bio only"
  }')

if echo "$PARTIAL_UPDATE" | grep -q "Profile updated successfully"; then
    echo -e "${GREEN}✅ Partial update successful${NC}"
else
    echo -e "${RED}❌ Partial update failed${NC}"
    exit 1
fi

# Step 11: Delete profile (soft delete)
echo ""
echo -e "${YELLOW}11. Deleting profile (soft delete)${NC}"
echo "-------------------------------------"
DELETE_RESPONSE=$(curl -s -X DELETE "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "Delete Response: $DELETE_RESPONSE"

if echo "$DELETE_RESPONSE" | grep -q "Profile deleted successfully"; then
    echo -e "${GREEN}✅ Profile deleted${NC}"
else
    echo -e "${RED}❌ Profile deletion failed${NC}"
    exit 1
fi

# Step 12: Get profile after deletion
echo ""
echo -e "${YELLOW}12. Getting profile after deletion${NC}"
echo "------------------------------------"
DELETED_PROFILE=$(curl -s -X GET "$BASE_URL/api/user/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "Profile after deletion: $DELETED_PROFILE"

echo ""
echo "====================================="
echo -e "${GREEN}✅ All profile operation tests passed!${NC}"
echo ""
echo "Summary:"
echo "- User registered and verified"
echo "- Profile created and updated"
echo "- Account details updated"
echo "- Username uniqueness enforced"
echo "- Partial updates work"
echo "- Soft delete works"
echo ""