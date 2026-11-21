DROP TABLE recovery_codes;
DROP TABLE user_notes;
ALTER TABLE refresh_tokens DROP COLUMN last_used_at, DROP COLUMN device_label, DROP COLUMN ip, DROP COLUMN user_agent;
ALTER TABLE members DROP COLUMN avatar_url, DROP COLUMN nickname;
ALTER TABLE users
  DROP COLUMN email_verified,
  DROP COLUMN two_factor_enabled,
  DROP COLUMN totp_secret,
  DROP COLUMN dnd,
  DROP COLUMN status_expires_at,
  DROP COLUMN status_emoji,
  DROP COLUMN status_text,
  DROP COLUMN accent_color,
  DROP COLUMN pronouns,
  DROP COLUMN banner_url,
  DROP COLUMN about;
