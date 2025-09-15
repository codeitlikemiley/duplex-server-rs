-- Drop tables in reverse order
DROP TABLE IF EXISTS unlock_attempts;
DROP TABLE IF EXISTS account_unlocks;
DROP TABLE IF EXISTS account_lockouts;

-- Drop lockout_type enum
DROP TYPE IF EXISTS lockout_type;
