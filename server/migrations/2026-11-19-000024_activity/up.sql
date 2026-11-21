ALTER TABLE users
  ADD COLUMN activity_name TEXT NOT NULL DEFAULT '',
  ADD COLUMN activity_type TEXT NOT NULL DEFAULT 'playing';
