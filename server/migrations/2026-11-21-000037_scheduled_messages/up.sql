CREATE TABLE scheduled_messages (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    author_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id   BIGINT NOT NULL,
    content      TEXT NOT NULL,
    scheduled_at TIMESTAMPTZ NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    sent_at      TIMESTAMPTZ,
    error_msg    TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX scheduled_messages_status_at_idx ON scheduled_messages(status, scheduled_at) WHERE status = 'pending';
CREATE INDEX scheduled_messages_channel_idx ON scheduled_messages(channel_id);
