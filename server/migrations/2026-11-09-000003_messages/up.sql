CREATE TABLE messages (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id  BIGINT NOT NULL REFERENCES users(id),
    content    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at  TIMESTAMPTZ
);
-- Курсорная пагинация: WHERE channel_id = ? AND id < ? ORDER BY id DESC.
CREATE INDEX messages_channel_id_id_idx ON messages(channel_id, id DESC);
