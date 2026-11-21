ALTER TABLE servers
  ADD COLUMN description TEXT NOT NULL DEFAULT '',
  ADD COLUMN rules TEXT NOT NULL DEFAULT '',
  ADD COLUMN banner_url TEXT NOT NULL DEFAULT '',
  ADD COLUMN announcement_channel_id BIGINT REFERENCES channels(id) ON DELETE SET NULL;

CREATE TABLE reaction_roles (
  message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  emoji TEXT NOT NULL,
  role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  PRIMARY KEY (message_id, emoji, role_id)
);
