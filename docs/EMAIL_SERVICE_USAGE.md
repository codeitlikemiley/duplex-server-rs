# Email Service Usage Guide

## Quick Start

### 1. Development Mode (Console Logging)
```bash
# Just run the app - console mode is the default
cargo run

# Or explicitly set it
EMAIL_SERVICE_TYPE=console cargo run
```

**What it does:** Logs all emails to console with tokens and links visible for testing

### 2. Production Mode (SMTP)
```bash
# Set these environment variables
EMAIL_SERVICE_TYPE=smtp
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-specific-password  # Use app password, not regular password
SMTP_FROM_EMAIL=noreply@yourapp.com
SMTP_FROM_NAME="Your App Name"
SMTP_USE_TLS=true

cargo run
```

### 3. Production Mode (SendGrid)
```bash
# Set these environment variables
EMAIL_SERVICE_TYPE=sendgrid
SENDGRID_API_KEY=SG.xxxxxxxxxxxxxxxxxxxxx
SENDGRID_FROM_EMAIL=noreply@yourapp.com
SENDGRID_FROM_NAME="Your App Name"

cargo run
```

## Testing the Email Flow

### Step 1: Register a User
```bash
curl -X POST http://localhost:80/api/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "Test123!",
    "first_name": "Test",
    "last_name": "User"
  }'
```

### Step 2: Get Verification Token (Dev Mode)
Look at the console output for:
```
📧 VERIFICATION TOKEN: ioVFdHDvDaeoNoBrAKNwCxaW8zR32vNtJ56y5WABQV7EtxYHbeNGjwV2ES7XLInT
```

### Step 3: Verify Email
```bash
curl -X POST http://localhost:80/api/verify-email \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "USER_ID_FROM_CONSOLE",
    "verification_token": "TOKEN_FROM_CONSOLE"
  }'
```

### Step 4: Login
```bash
curl -X POST http://localhost:80/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "Test123!"
  }'
```

## Environment Variables (.env file)

```env
# Database
DATABASE_URL=postgres://username@localhost:5432/yourdb

# Email Service Configuration
EMAIL_SERVICE_TYPE=console  # Options: console, smtp, sendgrid
EMAIL_LOG_FULL_CONTENT=false  # Set to true to see full email body in logs
BASE_URL=http://localhost:80
FROM_EMAIL=noreply@yourapp.com

# SMTP Configuration (when EMAIL_SERVICE_TYPE=smtp)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=noreply@yourapp.com
SMTP_FROM_NAME=Your App
SMTP_USE_TLS=true

# SendGrid Configuration (when EMAIL_SERVICE_TYPE=sendgrid)
SENDGRID_API_KEY=your-sendgrid-api-key
SENDGRID_FROM_EMAIL=noreply@yourapp.com
SENDGRID_FROM_NAME=Your App
```

## Adding a New Email Provider

1. Create a new file in `src/infrastructure/email/`:
```rust
// src/infrastructure/email/your_provider.rs
use async_trait::async_trait;
use super::email_service::{EmailService, EmailMessage, EmailError};

pub struct YourEmailProvider {
    // Your config
}

#[async_trait]
impl EmailService for YourEmailProvider {
    async fn send_email(&self, message: EmailMessage) -> Result<(), EmailError> {
        // Your implementation
    }

    // Implement other required methods...
}
```

2. Add to the factory in `src/infrastructure/email/email_factory.rs`:
```rust
EmailServiceType::YourProvider => {
    let provider = YourEmailProvider::new(/* config */);
    Arc::new(provider) as Arc<dyn EmailService>
}
```

3. Update the enum in `email_factory.rs`:
```rust
pub enum EmailServiceType {
    Console,
    Smtp,
    SendGrid,
    YourProvider,  // Add this
}
```

## Common Issues & Solutions

### Gmail SMTP Not Working?
1. Enable 2-factor authentication
2. Generate an app-specific password
3. Use the app password, not your regular password

### Emails Not Sending in Production?
Check your logs for errors:
```bash
# Enable full email logging temporarily
EMAIL_LOG_FULL_CONTENT=true cargo run
```

### Need to Test Without Actually Sending Emails?
Use console mode:
```bash
EMAIL_SERVICE_TYPE=console cargo run
```

### Want to See Full Email Content in Logs?
```bash
EMAIL_LOG_FULL_CONTENT=true cargo run
```

## Available Email Templates

The system includes pre-built templates for:
- **Email Verification** - Sent on user registration
- **Password Reset** - For password recovery flow
- **Welcome Email** - After successful verification
- **Password Changed** - Security notification

Templates are defined in `src/infrastructure/email/email_service.rs` in the `EmailTemplates` struct.

## Security Notes

1. **Never commit credentials** - Use environment variables
2. **Use app passwords** - Not your actual email password
3. **Validate email addresses** - The service includes address validation
4. **Rate limiting** - Implement rate limiting for production
5. **Token expiration** - Verification tokens expire after 24 hours

## Production Checklist

- [ ] Set `EMAIL_SERVICE_TYPE` to `smtp` or `sendgrid`
- [ ] Configure all required environment variables
- [ ] Test email delivery with a real email address
- [ ] Set up proper FROM email with SPF/DKIM records
- [ ] Implement rate limiting on registration endpoint
- [ ] Set up email bounce handling
- [ ] Monitor email delivery rates
- [ ] Set `EMAIL_LOG_FULL_CONTENT=false` in production

## Quick Test Script

Save this as `test_email.sh`:
```bash
#!/bin/bash

# Generate unique test data
TIMESTAMP=$(date +%s)
EMAIL="test_${TIMESTAMP}@example.com"
USERNAME="testuser_${TIMESTAMP}"

echo "Testing with email: $EMAIL"

# Register
echo "1. Registering user..."
curl -X POST http://localhost:80/api/register \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"$USERNAME\",
    \"email\": \"$EMAIL\",
    \"password\": \"Test123!\",
    \"first_name\": \"Test\",
    \"last_name\": \"User\"
  }"

echo -e "\n\n2. Check console for verification token and user ID"
echo "3. Run verification with: curl -X POST http://localhost:80/api/verify-email ..."
```

That's it. The email service is ready to use. Just set your environment variables and go.