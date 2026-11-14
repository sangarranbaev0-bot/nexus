ALTER TABLE messages ADD COLUMN pinned_at TIMESTAMPTZ;

CREATE INDEX messages_pinned_idx ON messages(channel_id, pinned_at) WHERE pinned_at IS NOT NULL;