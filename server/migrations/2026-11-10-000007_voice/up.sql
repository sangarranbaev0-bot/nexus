ALTER TABLE channels ADD COLUMN kind TEXT NOT NULL DEFAULT 'text' CHECK (kind IN ('text', 'voice'));

-- Кто сейчас в голосе. Один голосовой канал на пользователя (как в Discord): вход из другой
-- вкладки переносит его туда. Строка живёт, пока живо WebSocket-подключение.
CREATE TABLE voice_states (
    user_id       BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    channel_id    BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    connection_id BIGINT NOT NULL REFERENCES presence_connections(id) ON DELETE CASCADE,
    muted         BOOLEAN NOT NULL DEFAULT false,
    deafened      BOOLEAN NOT NULL DEFAULT false,
    camera        BOOLEAN NOT NULL DEFAULT false,
    screen        BOOLEAN NOT NULL DEFAULT false,
    joined_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX voice_states_channel_id_idx ON voice_states(channel_id);

-- WebRTC-сигналы (SDP, ICE) между собеседниками. Через таблицу, а не прямо в NOTIFY:
-- описание сессии с видео не помещается в лимит NOTIFY (8000 байт). Живут минуту.
CREATE TABLE voice_signals (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    from_user  BIGINT NOT NULL,
    to_user    BIGINT NOT NULL,
    payload    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
