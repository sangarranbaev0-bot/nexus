-- Пересылка сообщений: ссылка на оригинал плюс снимок автора и текста,
-- чтобы блок «Переслано от» пережил удаление исходного сообщения.
ALTER TABLE messages ADD COLUMN forwarded_from_id BIGINT REFERENCES messages(id) ON DELETE SET NULL;
ALTER TABLE messages ADD COLUMN forwarded_channel_id BIGINT;
ALTER TABLE messages ADD COLUMN forwarded_author_id BIGINT;
ALTER TABLE messages ADD COLUMN forwarded_author TEXT;
ALTER TABLE messages ADD COLUMN forwarded_content TEXT;
