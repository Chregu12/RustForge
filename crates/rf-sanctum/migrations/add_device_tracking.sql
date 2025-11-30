-- Add device tracking fields to personal_access_tokens table
ALTER TABLE personal_access_tokens
ADD COLUMN IF NOT EXISTS user_agent TEXT,
ADD COLUMN IF NOT EXISTS last_used_ip VARCHAR(45);

-- Create index for IP lookups (useful for security audits)
CREATE INDEX IF NOT EXISTS idx_personal_access_tokens_last_used_ip ON personal_access_tokens(last_used_ip);

-- Add comment explaining the IP column size (45 chars is enough for IPv6)
COMMENT ON COLUMN personal_access_tokens.last_used_ip IS 'Last IP address used with this token (supports IPv4 and IPv6)';
COMMENT ON COLUMN personal_access_tokens.user_agent IS 'User agent string of the device that created/uses this token';
