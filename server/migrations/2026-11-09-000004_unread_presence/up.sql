-- До какого сообщения пользователь дочитал канал. Нет строки — читал всё до вступления.
CREATE TABLE read_states (
    user_id              BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id           BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_message_id BIGINT NOT NULL,
    PRIMARY KEY (user_id, channel_id)
);

-- Живые WebSocket-подключения всех экземпляров. Строка без свежего пульса считается мёртвой.
CREATE TABLE presence_connections (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    idle         BOOLEAN NOT NULL DEFAULT false,
    heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX presence_connections_user_id_idx ON presence_connections(user_id);
CREATE INDEX presence_connections_heartbeat_at_idx ON presence_connections(heartbeat_at);
