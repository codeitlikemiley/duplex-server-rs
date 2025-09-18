# Email Service Configuration Guide

This application supports multiple email service providers to send transactional emails for user registration, password resets, and notifications.

## Supported Email Services

### 1. Console (Development)
Prints emails to the console/logs. Perfect for local development.

```env
EMAIL_SERVICE=console
```

### 2. SMTP (Production)
Send emails via any SMTP server (Gmail, Outlook, custom mail server).

```env
EMAIL_SERVICE=smtp
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=noreply@yourdomain.com
SMTP_FROM_NAME=Your App Name
SMTP_USE_TLS=true
```

#### Gmail Setup
1. Enable 2-factor authentication in your Google account
2. Generate an app-specific password: https://myaccount.google.com/apppasswords
3. Use your Gmail address as `SMTP_USERNAME`
4. Use the app password as `SMTP_PASSWORD`

### 3. SendGrid (Production)
Use SendGrid's API for reliable email delivery at scale.

```env
EMAIL_SERVICE=sendgrid
SENDGRID_API_KEY=SG.your-sendgrid-api-key
SENDGRID_FROM_EMAIL=noreply@yourdomain.com
SENDGRID_FROM_NAME=Your App Name
```

#### SendGrid Setup
1. Create a SendGrid account: https://sendgrid.com
2. Verify your sender domain/email
3. Generate an API key with "Mail Send" permissions
4. Add the API key to your environment variables

## Email Templates

The application sends the following email types:
- **Registration Verification**: Confirms user email address
- **Password Reset**: Allows users to reset forgotten passwords
- **Welcome Email**: Sent after successful verification
- **Account Notifications**: Security alerts and updates

## Testing Email Configuration

Run the application and register a new user:

```bash
# Set environment to use console output
export EMAIL_SERVICE=console

# Run the application
cargo run

# Test registration (in another terminal)
curl -X POST http://localhost:80/api/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "SecurePass123!"
  }'
```

You should see the verification email in the console output.

## Production Recommendations

1. **Use SendGrid or SMTP** in production, not console
2. **Enable TLS** for SMTP connections
3. **Verify sender domains** to improve deliverability
4. **Set up SPF, DKIM, and DMARC** records
5. **Monitor bounce rates** and handle unsubscribes
6. **Implement rate limiting** to prevent abuse

## Troubleshooting

### SMTP Connection Failed
- Check firewall rules for SMTP ports (25, 465, 587)
- Verify credentials are correct
- Ensure TLS settings match server requirements

### SendGrid API Errors
- Verify API key has correct permissions
- Check sender domain is verified
- Review SendGrid dashboard for account limits

### Emails Not Received
- Check spam/junk folders
- Verify email addresses are valid
- Review application logs for send errors
- Check provider's sending limits

## Environment Variables Reference

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| EMAIL_SERVICE | Service type: console, smtp, sendgrid | console | No |
| SMTP_HOST | SMTP server hostname | - | If smtp |
| SMTP_PORT | SMTP server port | 587 | If smtp |
| SMTP_USERNAME | SMTP authentication username | - | If smtp |
| SMTP_PASSWORD | SMTP authentication password | - | If smtp |
| SMTP_FROM_EMAIL | Sender email address | noreply@quake.app | No |
| SMTP_FROM_NAME | Sender display name | Quake App | No |
| SMTP_USE_TLS | Enable TLS encryption | true | No |
| SENDGRID_API_KEY | SendGrid API key | - | If sendgrid |
| SENDGRID_FROM_EMAIL | Sender email for SendGrid | noreply@quake.app | No |
| SENDGRID_FROM_NAME | Sender name for SendGrid | Quake App | No |