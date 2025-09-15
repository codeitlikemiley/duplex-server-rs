-- Create roles table
CREATE TABLE roles (
    id UUID PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    is_system BOOLEAN NOT NULL DEFAULT false, -- System roles cannot be deleted
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create permissions table
CREATE TABLE permissions (
    id UUID PRIMARY KEY,
    resource VARCHAR(100) NOT NULL, -- e.g., "users", "posts", "admin"
    action VARCHAR(50) NOT NULL,    -- e.g., "read", "write", "delete", "manage"
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(resource, action)
);

-- Create role_permissions junction table
CREATE TABLE role_permissions (
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    granted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    granted_by UUID REFERENCES users(id),
    PRIMARY KEY (role_id, permission_id)
);

-- Create user_roles junction table
CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id),
    expires_at TIMESTAMP WITH TIME ZONE, -- Optional: for temporary role assignments
    PRIMARY KEY (user_id, role_id)
);

-- Create indexes for efficient queries
CREATE INDEX idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX idx_user_roles_role_id ON user_roles(role_id);
CREATE INDEX idx_user_roles_expires_at ON user_roles(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission_id ON role_permissions(permission_id);
CREATE INDEX idx_permissions_resource ON permissions(resource);
CREATE INDEX idx_permissions_action ON permissions(action);

-- Insert default system roles
INSERT INTO roles (id, name, description, is_system) VALUES
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'super_admin', 'Full system access with all permissions', true),
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', 'admin', 'Administrative access with most permissions', true),
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'moderator', 'Can moderate content and users', true),
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'user', 'Regular user with basic permissions', true),
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', 'guest', 'Limited read-only access', true);

-- Insert default permissions
INSERT INTO permissions (id, resource, action, description) VALUES
    -- User permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a01', 'users', 'read', 'View user profiles'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a02', 'users', 'write', 'Create and update users'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a03', 'users', 'delete', 'Delete users'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a04', 'users', 'manage', 'Full user management'),

    -- Profile permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a05', 'profiles', 'read', 'View profiles'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a06', 'profiles', 'write', 'Update profiles'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a07', 'profiles', 'delete', 'Delete profiles'),

    -- Admin permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a08', 'admin', 'access', 'Access admin panel'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a09', 'admin', 'manage_roles', 'Manage roles and permissions'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a10', 'admin', 'view_logs', 'View system logs'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'admin', 'manage_settings', 'Manage system settings'),

    -- Session permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', 'sessions', 'read', 'View sessions'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'sessions', 'revoke', 'Revoke sessions'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'sessions', 'manage', 'Full session management'),

    -- Activity permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', 'activities', 'read', 'View activity logs'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a16', 'activities', 'manage', 'Manage activity logs'),

    -- Lockout permissions
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17', 'lockouts', 'read', 'View lockout status'),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18', 'lockouts', 'manage', 'Manage account lockouts');

-- Assign permissions to roles
-- Super Admin gets everything
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', id FROM permissions;

-- Admin gets most permissions (except some super admin only)
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', id FROM permissions
WHERE resource != 'admin' OR action != 'manage_settings';

-- Moderator permissions
INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a01'), -- users:read
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a05'), -- profiles:read
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15'), -- activities:read
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a17'), -- lockouts:read
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a13', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a18'); -- lockouts:manage

-- Regular user permissions
INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a01'), -- users:read (own profile)
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a05'), -- profiles:read
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a06'), -- profiles:write (own)
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12'); -- sessions:read (own)

-- Guest permissions (minimal)
INSERT INTO role_permissions (role_id, permission_id) VALUES
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a15', 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a01'); -- users:read (limited)

-- Function to check if a user has a specific permission
CREATE OR REPLACE FUNCTION user_has_permission(
    p_user_id UUID,
    p_resource VARCHAR(100),
    p_action VARCHAR(50)
) RETURNS BOOLEAN AS $$
BEGIN
    RETURN EXISTS (
        SELECT 1
        FROM user_roles ur
        JOIN role_permissions rp ON ur.role_id = rp.role_id
        JOIN permissions p ON rp.permission_id = p.id
        WHERE ur.user_id = p_user_id
            AND p.resource = p_resource
            AND p.action = p_action
            AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
    );
END;
$$ LANGUAGE plpgsql;

-- Function to get all permissions for a user
CREATE OR REPLACE FUNCTION get_user_permissions(p_user_id UUID)
RETURNS TABLE(resource VARCHAR(100), action VARCHAR(50)) AS $$
BEGIN
    RETURN QUERY
    SELECT DISTINCT p.resource, p.action
    FROM user_roles ur
    JOIN role_permissions rp ON ur.role_id = rp.role_id
    JOIN permissions p ON rp.permission_id = p.id
    WHERE ur.user_id = p_user_id
        AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
    ORDER BY p.resource, p.action;
END;
$$ LANGUAGE plpgsql;