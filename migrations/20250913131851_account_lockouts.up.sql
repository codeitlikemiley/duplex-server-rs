-- Create lockout_type enum
CREATE TYPE lockout_type AS ENUM ('Temporary', 'Permanent');

-- Create account_lockouts table
CREATE TABLE account_lockouts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lockout_type lockout_type NOT NULL,
    reason TEXT NOT NULL,
    locked_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    locked_until TIMESTAMP WITH TIME ZONE,
    unlock_attempts INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create account_unlocks table
CREATE TABLE account_unlocks (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    unlocked_by UUID NOT NULL REFERENCES users(id),
    unlocked_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create unlock_attempts table (for tracking failed unlock attempts)
CREATE TABLE unlock_attempts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ip_address TEXT,
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient lockout queries
CREATE INDEX idx_account_lockouts_user_id ON account_lockouts(user_id);
CREATE INDEX idx_account_lockouts_active ON account_lockouts(user_id, is_active) WHERE is_active = true;
CREATE INDEX idx_account_lockouts_temporary_expired ON account_lockouts(lockout_type, locked_until, is_active) WHERE lockout_type = 'Temporary' AND is_active = true;
CREATE INDEX idx_account_lockouts_created_at ON account_lockouts(created_at);

CREATE INDEX idx_account_unlocks_user_id ON account_unlocks(user_id);
CREATE INDEX idx_account_unlocks_unlocked_at ON account_unlocks(unlocked_at);

CREATE INDEX idx_unlock_attempts_user_id ON unlock_attempts(user_id);
CREATE INDEX idx_unlock_attempts_attempted_at ON unlock_attempts(attempted_at);
