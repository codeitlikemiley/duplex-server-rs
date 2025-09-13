-- Drop user sessions indexes
DROP INDEX IF EXISTS idx_sessions_revoked;
DROP INDEX IF EXISTS idx_sessions_expires_at;
DROP INDEX IF EXISTS idx_sessions_token_hash;
DROP INDEX IF EXISTS idx_sessions_user_id;

-- Drop user sessions table
DROP TABLE IF EXISTS user_sessions;