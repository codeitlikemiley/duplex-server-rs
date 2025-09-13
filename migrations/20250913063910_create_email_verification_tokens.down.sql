-- Drop email verification tokens indexes
DROP INDEX IF EXISTS idx_verification_tokens_expires_at;
DROP INDEX IF EXISTS idx_verification_tokens_token;
DROP INDEX IF EXISTS idx_verification_tokens_user_id;

-- Drop email verification tokens table
DROP TABLE IF EXISTS email_verification_tokens;