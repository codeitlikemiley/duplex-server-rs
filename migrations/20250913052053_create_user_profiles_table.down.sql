-- Drop user_profiles indexes
DROP INDEX IF EXISTS idx_user_profiles_last_name;
DROP INDEX IF EXISTS idx_user_profiles_first_name;

-- Drop user_profiles table
DROP TABLE IF EXISTS user_profiles;
