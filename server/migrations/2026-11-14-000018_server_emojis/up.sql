-- Свои эмодзи сервера: имя для :name: и PNG-файл (маленькая картинка до 256 КБ).
CREATE TABLE server_emojis (
    id BIGSERIAL PRIMARY KEY,
    server_id BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (name ~ '^[a-z0-9_]{2,32}$'),
    content_type TEXT NOT NULL CHECK (content_type = 'image/png'),
    size_bytes BIGINT NOT NULL CHECK (size_bytes > 0 AND size_bytes <= 262144),
    data BYTEA NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (server_id, name)
);

CREATE INDEX server_emojis_server_idx ON server_emojis(server_id);
