-- Центр уведомлений: упоминания, ответы и сработавшие слова-триггеры.
CREATE TABLE notifications (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('mention', 'reply', 'word', 'everyone')),
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    actor_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ,
    CONSTRAINT notifications_not_self CHECK (user_id <> actor_id)
);

CREATE INDEX notifications_user_idx ON notifications(user_id, created_at DESC);
CREATE INDEX notifications_unread_idx ON notifications(user_id) WHERE read_at IS NULL;

-- Глушить @everyone для этой настройки (сервера или канала).
ALTER TABLE notify_settings ADD COLUMN suppress_everyone BOOLEAN NOT NULL DEFAULT false;

-- Слова-триггеры: пользователь получает уведомление, когда в чужом сообщении есть слово.
CREATE TABLE notification_words (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    case_sensitive BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, word),
    CONSTRAINT notification_words_len CHECK (char_length(word) BETWEEN 2 AND 32)
);
