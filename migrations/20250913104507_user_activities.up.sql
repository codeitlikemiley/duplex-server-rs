-- Create activity_type enum
CREATE TYPE activity_type AS ENUM (
    'Login',
    'Logout',
    'PasswordChange',
    'ProfileUpdate',
    'AccountDeactivation',
    'AccountReactivation',
    'AccountDeletion',
    'EmailVerification',
    'PasswordReset',
    'SessionExpired',
    'SecurityAlert'
);

-- Create user_activities table
CREATE TABLE user_activities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_type activity_type NOT NULL,
    description TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_user_activities_user_id ON user_activities(user_id);
CREATE INDEX idx_user_activities_created_at ON user_activities(created_at);
CREATE INDEX idx_user_activities_activity_type ON user_activities(activity_type);
CREATE INDEX idx_user_activities_user_id_created_at ON user_activities(user_id, created_at DESC);
CREATE INDEX idx_user_activities_ip_address ON user_activities(ip_address) WHERE ip_address IS NOT NULL;
