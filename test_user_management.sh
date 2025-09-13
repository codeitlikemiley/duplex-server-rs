#!/bin/bash

# User Management API Test Script
# This script tests all user management endpoints

BASE_URL="http://localhost:80"
EMAIL="test_$(date +%s)@example.com"
USERNAME="testuser_$(date +%s)"
PASSWORD="TestPassword123!"
NEW_PASSWORD="NewPassword456!"

echo "🧪 Testing User Management API"
echo "================================"
echo "Base URL: $BASE_URL"
echo "Test Email: $EMAIL"
echo "Test Username: $USERNAME"
echo ""

# Color codes for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 1. Test User Registration
echo -e "${YELLOW}1. Testing User Registration${NC}"
echo "------------------------------"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/register" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "'$USERNAME'",
    "email": "'$EMAIL'",
    "password": "'$PASSWORD'",
    "first_name": "Test",
    "last_name": "User"
  }')

echo "Response: $REGISTER_RESPONSE"
if [[ $REGISTER_RESPONSE == *"Registration successful"* ]]; then
  echo -e "${GREEN}✅ Registration successful${NC}"
else
  echo -e "${RED}❌ Registration failed${NC}"
fi
echo ""

# 2. Test Login (should fail - email not verified)
echo -e "${YELLOW}2. Testing Login (before email verification)${NC}"
echo "---------------------------------------------"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "'$EMAIL'",
    "password": "'$PASSWORD'"
  }')

echo "Response: $LOGIN_RESPONSE"
if [[ $LOGIN_RESPONSE == *"verify your email"* ]]; then
  echo -e "${GREEN}✅ Correctly blocked unverified login${NC}"
else
  echo -e "${RED}❌ Unexpected response${NC}"
fi
echo ""

# 3. Get verification token from logs (in real app, this would be sent via email)
echo -e "${YELLOW}3. Getting Verification Token${NC}"
echo "------------------------------"
echo "In a real application, the verification token would be sent via email."
echo "For testing, check the application logs for the verification token."
echo "Look for a line like: 'Verification token created for user $EMAIL: <TOKEN>'"
echo ""
echo "Please enter the verification token from the logs:"
read -r VERIFICATION_TOKEN

# Parse user ID from create user response (you might need to adjust this)
echo "Please enter the user ID (UUID) from the logs:"
read -r USER_ID

# 4. Test Email Verification
echo -e "${YELLOW}4. Testing Email Verification${NC}"
echo "------------------------------"
VERIFY_RESPONSE=$(curl -s -X POST "$BASE_URL/api/verify-email" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "'$USER_ID'",
    "verification_token": "'$VERIFICATION_TOKEN'"
  }')

echo "Response: $VERIFY_RESPONSE"
if [[ $VERIFY_RESPONSE == *"verified successfully"* ]]; then
  echo -e "${GREEN}✅ Email verification successful${NC}"
else
  echo -e "${RED}❌ Email verification failed${NC}"
fi
echo ""

# 5. Test Login (should succeed after verification)
echo -e "${YELLOW}5. Testing Login (after email verification)${NC}"
echo "--------------------------------------------"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "'$EMAIL'",
    "password": "'$PASSWORD'"
  }')

echo "Response: $LOGIN_RESPONSE"
if [[ $LOGIN_RESPONSE == *"token"* ]]; then
  echo -e "${GREEN}✅ Login successful${NC}"
  # Extract token from response
  TOKEN=$(echo $LOGIN_RESPONSE | grep -o '"token":"[^"]*' | sed 's/"token":"//')
  echo "JWT Token: $TOKEN"
else
  echo -e "${RED}❌ Login failed${NC}"
  exit 1
fi
echo ""

# 6. Test Protected Endpoint (Profile)
echo -e "${YELLOW}6. Testing Protected Endpoint (Profile)${NC}"
echo "---------------------------------------"
PROFILE_RESPONSE=$(curl -s -X GET "$BASE_URL/api/profile" \
  -H "Authorization: Bearer $TOKEN")

echo "Response: $PROFILE_RESPONSE"
if [[ $PROFILE_RESPONSE == *"user_id"* ]]; then
  echo -e "${GREEN}✅ Profile access successful${NC}"
else
  echo -e "${RED}❌ Profile access failed${NC}"
fi
echo ""

# 7. Test Password Change
echo -e "${YELLOW}7. Testing Password Change${NC}"
echo "--------------------------"
CHANGE_PWD_RESPONSE=$(curl -s -X POST "$BASE_URL/api/change-password" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "current_password": "'$PASSWORD'",
    "new_password": "'$NEW_PASSWORD'"
  }')

echo "Response: $CHANGE_PWD_RESPONSE"
if [[ $CHANGE_PWD_RESPONSE == *"changed successfully"* ]]; then
  echo -e "${GREEN}✅ Password change successful${NC}"
else
  echo -e "${RED}❌ Password change failed${NC}"
fi
echo ""

# 8. Test Login with New Password
echo -e "${YELLOW}8. Testing Login with New Password${NC}"
echo "-----------------------------------"
NEW_LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "'$EMAIL'",
    "password": "'$NEW_PASSWORD'"
  }')

echo "Response: $NEW_LOGIN_RESPONSE"
if [[ $NEW_LOGIN_RESPONSE == *"token"* ]]; then
  echo -e "${GREEN}✅ Login with new password successful${NC}"
else
  echo -e "${RED}❌ Login with new password failed${NC}"
fi
echo ""

# 9. Test Get User by ID
echo -e "${YELLOW}9. Testing Get User by ID${NC}"
echo "-------------------------"
USER_RESPONSE=$(curl -s -X GET "$BASE_URL/api/users/$USER_ID")

echo "Response: $USER_RESPONSE"
if [[ $USER_RESPONSE == *"$EMAIL"* ]]; then
  echo -e "${GREEN}✅ Get user successful${NC}"
else
  echo -e "${RED}❌ Get user failed${NC}"
fi
echo ""

echo "================================"
echo -e "${GREEN}🎉 Test Suite Complete!${NC}"
echo "================================"
echo ""
echo "Summary of endpoints tested:"
echo "- POST /api/register - User registration"
echo "- POST /api/verify-email - Email verification"
echo "- POST /api/login - User login"
echo "- GET /api/profile - Protected profile endpoint"
echo "- POST /api/change-password - Password change"
echo "- GET /api/users/:id - Get user by ID"