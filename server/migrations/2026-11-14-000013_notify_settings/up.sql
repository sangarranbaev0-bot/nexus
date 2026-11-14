-- Настройка уведомлений пользователя для сервера или канала (ровно одно из двух).
-- level задаёт порог (все / только @упоминания / ничего), muted_until — временное подавление.
CREATE TABLE notify_settings (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id BIGINT REFERENCES servers(id) ON DELETE CASCADE,
    channel_id BIGINT REFERENCES channels(id) ON DELETE CASCADE,
    level TEXT NOT NULL CHECK (level IN ('all', 'mentions', 'none')),
    muted_until TIMESTAMPTZ,
    CHECK ((server_id IS NULL) <> (channel_id IS NULL))
);
CREATE UNIQUE INDEX notify_settings_server ON notify_settings (user_id, server_id) WHERE server_id IS NOT NULL;
CREATE UNIQUE INDEX notify_settings_channel ON notify_settings (user_id, channel_id) WHERE channel_id IS NOT NULL;
