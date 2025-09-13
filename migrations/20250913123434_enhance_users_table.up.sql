-- Add new fields to users table for enhanced user management
ALTER TABLE users ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN status VARCHAR(50) NOT NULL DEFAULT 'Active';
ALTER TABLE users ADD COLUMN created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();
ALTER TABLE users ADD COLUMN updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();
ALTER TABLE users ADD COLUMN last_login_at TIMESTAMP WITH TIME ZONE;

-- Update existing users to have proper timestamps
UPDATE users SET created_at = NOW(), updated_at = NOW(), email_verified = TRUE, status = 'Active' WHERE created_at IS NULL;

-- Create user_profiles table for extended user information
CREATE TABLE user_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    first_name VARCHAR(255),
    last_name VARCHAR(255),
    avatar_url VARCHAR(500),
    bio TEXT,
    preferences JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX idx_users_email_verified ON users(email_verified);
CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_users_created_at ON users(created_at);
CREATE INDEX idx_user_profiles_first_name ON user_profiles(first_name);
CREATE INDEX idx_user_profiles_last_name ON user_profiles(last_name);