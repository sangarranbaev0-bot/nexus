DROP TABLE dm_participants;
DELETE FROM channels WHERE server_id IS NULL;
ALTER TABLE channels DROP CONSTRAINT channels_server_or_dm;
ALTER TABLE channels DROP COLUMN dm_key;
ALTER TABLE channels ALTER COLUMN server_id SET NOT NULL;
