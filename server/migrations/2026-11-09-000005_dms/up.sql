-- ЛС — канал без сервера на двоих. Так лента, правка, «печатает…» и непрочитанные общие с серверами.
ALTER TABLE channels ALTER COLUMN server_id DROP NOT NULL;
-- "меньший_id:больший_id": один диалог на пару пользователей.
ALTER TABLE channels ADD COLUMN dm_key TEXT UNIQUE;
ALTER TABLE channels ADD CONSTRAINT channels_server_or_dm CHECK ((server_id IS NULL) = (dm_key IS NOT NULL));

CREATE TABLE dm_participants (
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (channel_id, user_id)
);
CREATE INDEX dm_participants_user_id_idx ON dm_participants(user_id);
