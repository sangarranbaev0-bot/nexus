-- Системный канал сервера: куда пишутся события (вход, создание канала, роли).
ALTER TABLE servers ADD COLUMN system_channel_id BIGINT REFERENCES channels(id) ON DELETE SET NULL;

-- Системные сообщения: `system_kind` не NULL у событийных записей, `system_detail` — имя/роль.
ALTER TABLE messages ADD COLUMN system_kind TEXT;
ALTER TABLE messages ADD COLUMN system_detail TEXT;
