ALTER TABLE messages ADD COLUMN reply_to_id BIGINT REFERENCES messages(id) ON DELETE SET NULL;

CREATE INDEX messages_reply_to_idx ON messages(reply_to_id);