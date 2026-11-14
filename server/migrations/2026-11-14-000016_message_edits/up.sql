-- История правок сообщений: предыдущее содержимое сохраняется при каждом изменении.
CREATE TABLE message_edits (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX message_edits_message_idx ON message_edits(message_id, edited_at DESC);
