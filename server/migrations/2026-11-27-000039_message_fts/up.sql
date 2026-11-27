CREATE INDEX IF NOT EXISTS messages_content_fts_idx ON messages USING GIN (to_tsvector('russian', content));
