-- Create auth_rate_limits table for tracking authentication attempts
CREATE TABLE auth_rate_limits (
    id UUID PRIMARY KEY,
    ip_address TEXT,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    username TEXT,
    success BOOLEAN NOT NULL DEFAULT false,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient rate limit queries
CREATE INDEX idx_auth_rate_limits_ip_created ON auth_rate_limits(ip_address, created_at DESC);
CREATE INDEX idx_auth_rate_limits_user_created ON auth_rate_limits(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX idx_auth_rate_limits_created_at ON auth_rate_limits(created_at);
CREATE INDEX idx_auth_rate_limits_ip_success ON auth_rate_limits(ip_address, success, created_at DESC);
CREATE INDEX idx_auth_rate_limits_user_success ON auth_rate_limits(user_id, success, created_at DESC) WHERE user_id IS NOT NULL;
