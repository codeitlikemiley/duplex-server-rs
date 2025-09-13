-- Rollback: Remove new fields from users table
ALTER TABLE users DROP COLUMN IF EXISTS email_verified;
ALTER TABLE users DROP COLUMN IF EXISTS status;
ALTER TABLE users DROP COLUMN IF EXISTS created_at;
ALTER TABLE users DROP COLUMN IF EXISTS updated_at;
ALTER TABLE users DROP COLUMN IF EXISTS last_login_at;

-- Drop user_profiles table
DROP TABLE IF EXISTS user_profiles;

-- Drop indexes
DROP INDEX IF EXISTS idx_users_email_verified;
DROP INDEX IF EXISTS idx_users_status;
DROP INDEX IF EXISTS idx_users_created_at;
DROP INDEX IF EXISTS idx_user_profiles_first_name;
DROP INDEX IF EXISTS idx_user_profiles_last_name;