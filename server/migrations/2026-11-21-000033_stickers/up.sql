CREATE TABLE sticker_packs (
    id          BIGSERIAL PRIMARY KEY,
    server_id   BIGINT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_by  BIGINT NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE stickers (
    id           BIGSERIAL PRIMARY KEY,
    pack_id      BIGINT NOT NULL REFERENCES sticker_packs(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    emoji        TEXT NOT NULL DEFAULT '',
    content_type TEXT NOT NULL DEFAULT 'image/webp',
    size_bytes   BIGINT NOT NULL DEFAULT 0,
    storage_key  TEXT NOT NULL,
    tags         TEXT[] NOT NULL DEFAULT '{}',
    position     INT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stickers_pack ON stickers(pack_id);
CREATE INDEX idx_stickers_tags ON stickers USING GIN(tags);
