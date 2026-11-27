CREATE TABLE drafts (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL,
    content    TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX drafts_user_channel_idx ON drafts(user_id, channel_id);
ALTER TABLE drafts ADD CONSTRAINT drafts_unique_user_channel UNIQUE (user_id, channel_id);
