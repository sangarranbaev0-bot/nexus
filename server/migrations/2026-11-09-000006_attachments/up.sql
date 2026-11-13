-- Файл живёт в S3, здесь — его описание. message_id пуст, пока файл не прикреплён к сообщению.
-- При удалении сообщения или канала ссылка обнуляется, а сам файл убирает фоновая уборка.
CREATE TABLE attachments (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    channel_id   BIGINT REFERENCES channels(id) ON DELETE SET NULL,
    message_id   BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    uploader_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL,
    storage_key  TEXT NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX attachments_message_id_idx ON attachments(message_id);
CREATE INDEX attachments_orphan_idx ON attachments(created_at) WHERE message_id IS NULL;
