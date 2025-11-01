-- Drop all auth-related tables in reverse order to respect foreign key constraints

DROP INDEX IF EXISTS idx_refresh_tokens_expires_at;
DROP INDEX IF EXISTS idx_refresh_tokens_token;
DROP INDEX IF EXISTS idx_refresh_tokens_user_id;
DROP TABLE IF EXISTS refresh_tokens;

DROP INDEX IF EXISTS idx_permission_role_permission_id;
DROP INDEX IF EXISTS idx_permission_role_role_id;
DROP TABLE IF EXISTS permission_role;

DROP INDEX IF EXISTS idx_role_user_role_id;
DROP INDEX IF EXISTS idx_role_user_user_id;
DROP TABLE IF EXISTS role_user;

DROP INDEX IF EXISTS idx_permissions_slug;
DROP TABLE IF EXISTS permissions;

DROP INDEX IF EXISTS idx_roles_slug;
DROP TABLE IF EXISTS roles;

DROP INDEX IF EXISTS idx_sessions_expires_at;
DROP INDEX IF EXISTS idx_sessions_last_activity;
DROP INDEX IF EXISTS idx_sessions_user_id;
DROP TABLE IF EXISTS sessions;

DROP INDEX IF EXISTS idx_users_active;
DROP INDEX IF EXISTS idx_users_email;
DROP TABLE IF EXISTS users;
